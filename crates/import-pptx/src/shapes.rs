//! Converting a shape tree (`p:spTree`) into objects.
use crate::color::*;
use crate::geom::{self, Preset};
use crate::text::*;
use crate::{Cx, Link, PhInfo};
use roxmltree::Node;
use brainbend_doc::*;

/// A frame in EMU.
#[derive(Clone, Copy, Debug, Default)]
pub struct Xfrm {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub rot: f32,
    pub flip_h: bool,
    pub flip_v: bool,
}

pub fn parse_xfrm(n: Node) -> Option<Xfrm> {
    let off = child(n, "off")?;
    let ext = child(n, "ext")?;
    Some(Xfrm {
        x: attr_f(off, "x")?,
        y: attr_f(off, "y")?,
        w: attr_f(ext, "cx")?,
        h: attr_f(ext, "cy")?,
        rot: attr_f(n, "rot").unwrap_or(0.0) / 60000.0,
        flip_h: n.attribute("flipH") == Some("1"),
        flip_v: n.attribute("flipV") == Some("1"),
    })
}

/// Maps a group's child coordinates to slide units.
#[derive(Clone, Copy, Debug)]
pub struct Tf {
    sx: f32,
    sy: f32,
    tx: f32,
    ty: f32,
}

impl Tf {
    pub fn base(k: f32) -> Self {
        Tf { sx: k, sy: k, tx: 0.0, ty: 0.0 }
    }
    fn apply(&self, x: &Xfrm) -> Xfrm {
        Xfrm { x: x.x * self.sx + self.tx, y: x.y * self.sy + self.ty, w: x.w * self.sx, h: x.h * self.sy, ..*x }
    }
    fn group(&self, off: (f32, f32), ext: (f32, f32), ch_off: (f32, f32), ch_ext: (f32, f32)) -> Tf {
        let (kx, ky) = (if ch_ext.0 > 0.0 { ext.0 / ch_ext.0 } else { 1.0 }, if ch_ext.1 > 0.0 { ext.1 / ch_ext.1 } else { 1.0 });
        Tf { sx: self.sx * kx, sy: self.sy * ky, tx: self.sx * (off.0 - ch_off.0 * kx) + self.tx, ty: self.sy * (off.1 - ch_off.1 * ky) + self.ty }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BodyPr {
    pub l: Option<f32>,
    pub t: Option<f32>,
    pub r: Option<f32>,
    pub b: Option<f32>,
    pub anchor: Option<VAlign>,
    pub vert: Option<i16>,
    pub font_scale: Option<f32>,
    /// Text shrinks to fit (`normAutofit`).
    pub shrink: bool,
}

pub fn parse_body_pr(n: Option<Node>) -> BodyPr {
    let Some(n) = n else { return BodyPr::default() };
    BodyPr {
        l: attr_f(n, "lIns"),
        t: attr_f(n, "tIns"),
        r: attr_f(n, "rIns"),
        b: attr_f(n, "bIns"),
        anchor: n.attribute("anchor").and_then(|a| match a { "t" => Some(VAlign::Top), "ctr" => Some(VAlign::Middle), "b" => Some(VAlign::Bottom), _ => None }),
        vert: n.attribute("vert").and_then(|v| match v { "vert" | "eaVert" => Some(90), "vert270" => Some(270), _ => None }),
        font_scale: child(n, "normAutofit").and_then(|f| attr_f(f, "fontScale")).map(|v| v / 100000.0),
        shrink: child(n, "normAutofit").is_some(),
    }
}

impl BodyPr {
    fn fill_from(&mut self, o: &BodyPr) {
        self.l = self.l.or(o.l);
        self.t = self.t.or(o.t);
        self.r = self.r.or(o.r);
        self.b = self.b.or(o.b);
        self.anchor = self.anchor.or(o.anchor);
        self.vert = self.vert.or(o.vert);
        self.font_scale = self.font_scale.or(o.font_scale);
        self.shrink = self.shrink || o.shrink;
    }
}

fn ph_of(sp: Node) -> Option<(String, Option<u32>)> {
    let ph = child(child(child(sp, "nvSpPr").or_else(|| child(sp, "nvPicPr")).or_else(|| child(sp, "nvGraphicFramePr"))?, "nvPr")?, "ph")?;
    Some((ph.attribute("type").unwrap_or("obj").to_string(), ph.attribute("idx").and_then(|v| v.parse().ok())))
}

/// Placeholders directly inside `tree` (a layout's or master's shape tree).
pub fn collect_placeholders(tree: Node, theme: &ThemeData) -> Vec<PhInfo> {
    let mut out = vec![];
    for sp in children(tree, "sp") {
        let Some((ty, idx)) = ph_of(sp) else { continue };
        let sp_pr = child(sp, "spPr");
        let tx = child(sp, "txBody");
        let ln = sp_pr.and_then(|p| child(p, "ln"));
        out.push(PhInfo {
            ty,
            idx,
            xfrm: sp_pr.and_then(|p| child(p, "xfrm")).and_then(parse_xfrm),
            lst: parse_lst(tx.and_then(|t| child(t, "lstStyle")), theme),
            body: parse_body_pr(tx.and_then(|t| child(t, "bodyPr"))),
            fill: sp_pr.and_then(parse_fill),
            line: ln.and_then(parse_fill),
            line_w: ln.and_then(|l| attr_f(l, "w")),
        });
    }
    out
}

fn kind_of(ty: &str) -> Option<PlaceholderKind> {
    match ty {
        "title" | "ctrTitle" => Some(PlaceholderKind::Title),
        "subTitle" => Some(PlaceholderKind::Subtitle),
        "body" | "obj" => Some(PlaceholderKind::Body),
        _ => None,
    }
}

pub fn convert_children(cx: &mut Cx, parent: Node, tf: Tf, out: &mut Vec<Object>) {
    for c in parent.children().filter(|c| c.is_element()) {
        let before = out.len();
        convert_one(cx, c, tf, out);
        // Remember which objects came from which source shape, for animations.
        let spid = match c.tag_name().name() {
            "sp" => child(c, "nvSpPr"),
            "pic" => child(c, "nvPicPr"),
            "cxnSp" => child(c, "nvCxnSpPr"),
            "grpSp" => child(c, "nvGrpSpPr"),
            "graphicFrame" => child(c, "nvGraphicFramePr"),
            _ => None,
        }.and_then(|n| child(n, "cNvPr")).and_then(|n| n.attribute("id"));
        if let Some(id) = spid {
            if out.len() > before { cx.sp_ids.insert(id.to_string(), out[before..].iter().map(|o| o.id.clone()).collect()); }
        }
    }
}

fn convert_one(cx: &mut Cx, c: Node, tf: Tf, out: &mut Vec<Object>) {
    {
        match c.tag_name().name() {
            "sp" => convert_sp(cx, c, tf, out),
            "pic" => convert_pic(cx, c, tf, out),
            "cxnSp" => convert_connector(cx, c, tf, out),
            "graphicFrame" => convert_frame(cx, c, tf, out),
            "grpSp" => {
                let xf = child(c, "grpSpPr").and_then(|p| child(p, "xfrm"));
                let g = xf.and_then(|x| {
                    let (off, ext) = (child(x, "off")?, child(x, "ext")?);
                    let (co, ce) = (child(x, "chOff")?, child(x, "chExt")?);
                    Some(tf.group((attr_f(off, "x")?, attr_f(off, "y")?), (attr_f(ext, "cx")?, attr_f(ext, "cy")?), (attr_f(co, "x")?, attr_f(co, "y")?), (attr_f(ce, "cx")?, attr_f(ce, "cy")?)))
                });
                if xf.and_then(|x| x.attribute("rot")).is_some_and(|r| r != "0") { cx.report.add("Rotated groups are shown unrotated."); }
                let before = out.len();
                convert_children(cx, c, g.unwrap_or(tf), out);
                // Keep the group: its objects select, move and resize together.
                let gid = cx.next_id() + "-grp";
                for o in out[before..].iter_mut().filter(|o| o.table.is_none()) { o.group = Some(gid.clone()); }
            }
            "AlternateContent" => {
                // Prefer the plain fallback (usually a picture) over the newer element.
                if let Some(f) = child(c, "Fallback") { convert_children(cx, f, tf, out); } else if let Some(ch) = child(c, "Choice") { convert_children(cx, ch, tf, out); }
            }
            _ => {}
        }
    }
}

struct TextCtx<'a> {
    /// `title`, `body` or `other` text styles of the master.
    family: &'static str,
    lph: Option<&'a PhInfo>,
    mph: Option<&'a PhInfo>,
    /// Default text colour from the shape's style reference.
    font_ref: Option<Rgba>,
    /// Forced text defaults (table header cells).
    force_bold: bool,
    force_color: Option<Rgba>,
}

fn resolve_ph<'a>(cx: &'a Cx, ph: &Option<(String, Option<u32>)>) -> (Option<&'a PhInfo>, Option<&'a PhInfo>) {
    match ph {
        Some((t, i)) => cx.ch.find_ph(t, *i),
        None => (None, None),
    }
}

fn ph_family(ph: &Option<(String, Option<u32>)>) -> &'static str {
    match ph.as_ref().map(|p| p.0.as_str()) {
        Some("title") | Some("ctrTitle") => "title",
        Some("dt") | Some("ftr") | Some("sldNum") | None => "other",
        _ => "body",
    }
}

/// Style, margins and text of `o` from `tx` (a `txBody`), through the full inheritance chain.
fn apply_text(cx: &mut Cx, o: &mut Object, tx: Option<Node>, tc: &TextCtx) {
    let theme = cx.ch.theme.clone();
    let own_lst = parse_lst(tx.and_then(|t| child(t, "lstStyle")), &theme);
    let mut body = parse_body_pr(tx.and_then(|t| child(t, "bodyPr")));
    if let Some(l) = tc.lph { body.fill_from(&l.body); }
    if let Some(m) = tc.mph { body.fill_from(&m.body); }
    let k = cx.k;
    o.margins = [body.l.unwrap_or(91440.0) * k, body.t.unwrap_or(45720.0) * k, body.r.unwrap_or(91440.0) * k, body.b.unwrap_or(45720.0) * k];
    o.style.valign = body.anchor.unwrap_or(VAlign::Top);
    o.text_rotation = body.vert.unwrap_or(0);
    let mut scale = body.font_scale.unwrap_or(1.0);

    let mut chain: Vec<&Lst> = vec![&own_lst];
    if let Some(l) = tc.lph { chain.push(&l.lst); }
    if let Some(m) = tc.mph { chain.push(&m.lst); }
    match tc.family {
        "title" => chain.push(&cx.ch.tx_title),
        "body" => chain.push(&cx.ch.tx_body),
        _ => {}
    }
    chain.push(&cx.ch.default_text);

    let paras: Vec<Node> = tx.map(|t| children(t, "p").collect()).unwrap_or_default();
    struct P {
        eff: ParaProps,
        lvl: usize,
        runs: Vec<(String, RunProps)>,
    }
    let mut parsed: Vec<P> = vec![];
    for p in &paras {
        let ppr = child(*p, "pPr");
        let lvl = ppr.and_then(|n| n.attribute("lvl")).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0).min(8);
        let own = ppr.map(|n| parse_para_props(n, &theme)).unwrap_or_default();
        let eff = effective(own, &chain, lvl);
        let mut runs = vec![];
        for r in p.children().filter(|c| c.is_element()) {
            match r.tag_name().name() {
                "r" | "fld" => {
                    let t: String = children(r, "t").filter_map(|t| t.text()).collect();
                    let rp = child(r, "rPr").map(|n| parse_run_props(n, &theme)).unwrap_or_default();
                    if r.tag_name().name() == "fld" && child(r, "t").is_none() { continue; }
                    runs.push((t, rp));
                }
                "br" => runs.push(("\n".into(), RunProps::default())),
                _ => {}
            }
        }
        parsed.push(P { eff, lvl, runs });
    }
    // PowerPoint shrinks text in boxes set to "shrink text on overflow" when it opens the file;
    // estimate the same so the text fits here too.
    if body.shrink && body.font_scale.is_none() {
        let (mut bw, mut bh) = ((o.w - o.margins[0] - o.margins[2]).max(1.0), (o.h - o.margins[1] - o.margins[3]).max(1.0));
        if o.text_rotation != 0 { std::mem::swap(&mut bw, &mut bh); }
        let items: Vec<(f32, usize)> = parsed.iter().map(|p| {
            let size = p.runs.iter().find_map(|(_, r)| r.sz).or(p.eff.def.sz).unwrap_or(18.0) * cx.fk;
            (size, p.runs.iter().map(|(t, _)| t.chars().count()).sum::<usize>() + 3 * p.lvl + 2)
        }).collect();
        let height = |s: f32| -> f32 {
            items.iter().map(|&(size, chars)| {
                let size = size * s;
                let per_line = (bw / (size * 0.52)).floor().max(1.0);
                ((chars as f32 / per_line).ceil().max(1.0)) * size * 1.2 + size * 0.2
            }).sum()
        };
        if height(1.0) > bh {
            let (mut lo, mut hi) = (0.4f32, 1.0f32);
            for _ in 0..12 { let mid = (lo + hi) / 2.0; if height(mid) > bh { hi = mid; } else { lo = mid; } }
            scale = lo;
        }
    }
    // Object style from the first paragraph (or the level-1 defaults when there is no text).
    let first = parsed.first().map(|p| p.eff.clone()).unwrap_or_else(|| effective(ParaProps::default(), &chain, 0));
    let pt = first.def.sz.unwrap_or(18.0) * scale;
    let default_color = tc.force_color.or(tc.font_ref).unwrap_or(Rgba(0, 0, 0, 255));
    let color_of = |c: &Option<ColorSpec>| c.as_ref().map(|c| cx.ch.color(c, None)).or(tc.force_color).unwrap_or(default_color);
    o.style = TextStyle {
        font_family: first.def.font.clone().unwrap_or_else(|| theme.minor.clone()),
        size: (pt * cx.fk).round().max(1.0),
        bold: first.def.b.unwrap_or(tc.force_bold),
        italic: first.def.i.unwrap_or(false),
        underline: first.def.u.unwrap_or(false),
        color: color_of(&first.def.color),
        align: first.algn.unwrap_or(Align::Left),
        valign: o.style.valign,
    };
    for f in std::iter::once(o.style.font_family.clone()).chain(parsed.iter().flat_map(|p| p.runs.iter().filter_map(|(_, r)| r.font.clone()))) {
        let safe = ["open sans", "montserrat", "arial", "helvetica", "times new roman", "georgia", "verdana", "tahoma", "courier new", "trebuchet ms"];
        if !safe.contains(&f.to_ascii_lowercase().as_str()) && !f.is_empty() {
            cx.report.font(&f);
        }
    }
    // 100% in PowerPoint is the font's own line height, about 1.2 em.
    o.line_spacing = first.line.filter(|l| (l - 1.0).abs() > 0.01).map(|l| (l * 1.2 * 100.0).round() / 100.0).unwrap_or(0.0);
    // Text with per-run differences.
    let mut paragraphs: Vec<Paragraph> = vec![];
    let mut counters = [0usize; 9];
    let mut any_override = false;
    let mut plain: Vec<String> = vec![];
    for p in &parsed {
        let has_text = p.runs.iter().any(|(t, _)| !t.trim().is_empty());
        let mut prefix = String::new();
        if has_text {
            prefix.push_str(&"\u{a0}".repeat(3 * p.lvl));
            match &p.eff.bullet {
                Some(Bullet::Char(c)) => { prefix.push_str(c); prefix.push(' '); counters[p.lvl] = 0; }
                Some(Bullet::Auto(kind)) => {
                    counters[p.lvl] += 1;
                    prefix.push_str(&auto_number(kind, counters[p.lvl]));
                    prefix.push(' ');
                }
                _ => counters[p.lvl] = 0,
            }
            for c in counters.iter_mut().skip(p.lvl + 1) { *c = 0; }
        }
        let mut runs: Vec<Run> = vec![];
        if !prefix.is_empty() { runs.push(Run { text: prefix.clone(), ..Default::default() }); }
        for (t, rp) in &p.runs {
            let mut eff = rp.clone();
            merge_run(&mut eff, &p.eff.def);
            let mut text = t.clone();
            if eff.cap_all == Some(true) { text = text.to_uppercase(); }
            let mut run = Run { text, ..Default::default() };
            let size = eff.sz.map(|s| (s * scale * cx.fk).round().max(1.0));
            if size.is_some_and(|s| (s - o.style.size).abs() > 0.5) { run.size = size; }
            if eff.b.is_some_and(|b| b != o.style.bold) { run.bold = eff.b; }
            if eff.i.is_some_and(|i| i != o.style.italic) { run.italic = eff.i; }
            if eff.u.is_some_and(|u| u != o.style.underline) { run.underline = eff.u; }
            if eff.strike == Some(true) { run.strike = Some(true); }
            if eff.spc.is_some_and(|z| z.abs() > 0.5) { run.spacing = eff.spc.map(|z| z / 50.0); }
            let col = color_of(&eff.color);
            if col != o.style.color { run.color = Some(col); }
            if eff.font.as_ref().is_some_and(|f| *f != o.style.font_family) { run.font_family = eff.font.clone(); }
            if let Some(rel) = rp.link_rid.as_ref().and_then(|id| cx.rels.get(id)) {
                let u = rel.target.trim();
                let lower = u.to_ascii_lowercase();
                if rel.external && (lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")) { run.link = Some(u.to_string()); }
            }
            if run.size.is_some() || run.bold.is_some() || run.italic.is_some() || run.underline.is_some() || run.color.is_some() || run.font_family.is_some() || run.link.is_some() || run.strike.is_some() || run.spacing.is_some() { any_override = true; }
            runs.push(run);
        }
        plain.push(runs.iter().map(|r| r.text.as_str()).collect());
        paragraphs.push(Paragraph { runs, ..Default::default() });
    }
    let text = plain.join("\n");
    if text.trim().is_empty() {
        o.text = String::new();
        o.rich = None;
    } else {
        o.text = text;
        o.rich = if any_override { Some(paragraphs) } else { None };
    }
}

fn adj_reader<'a>(geom: Option<Node<'a, 'a>>) -> impl Fn(&str, f32) -> f32 + 'a {
    move |name: &str, default: f32| {
        geom.and_then(|g| child(g, "avLst")).and_then(|l| children(l, "gd").find(|g| g.attribute("name") == Some(name)))
            .and_then(|g| g.attribute("fmla")).and_then(|f| f.strip_prefix("val ")).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(default)
    }
}

struct Style {
    fill_ref: Option<(usize, Option<ColorSpec>)>,
    ln_ref: Option<(usize, Option<ColorSpec>)>,
    font_color: Option<ColorSpec>,
}

fn read_style(sp: Node) -> Style {
    let st = child(sp, "style");
    let r = |name: &str| st.and_then(|s| child(s, name)).map(|n| (n.attribute("idx").and_then(|v| v.parse().ok()).unwrap_or(0usize), first_color(n)));
    Style { fill_ref: r("fillRef"), ln_ref: r("lnRef"), font_color: st.and_then(|s| child(s, "fontRef")).and_then(first_color) }
}

fn shape_fill(cx: &Cx, sp_pr: Option<Node>, style: &Style, lph: Option<&PhInfo>, mph: Option<&PhInfo>) -> Option<FillSpec> {
    if let Some(f) = sp_pr.and_then(parse_fill) { return Some(f); }
    if let Some(f) = lph.and_then(|p| p.fill.clone()).or_else(|| mph.and_then(|p| p.fill.clone())) { return Some(f); }
    let (idx, clr) = style.fill_ref.clone()?;
    if idx == 0 { return None; }
    let spec = if idx >= 1001 { cx.ch.theme.bg_fills.get(idx - 1001) } else { cx.ch.theme.fills.get(idx - 1) }?.clone();
    // A theme fill uses the referencing colour as its base.
    let ph = clr.map(|c| cx.ch.color(&c, None));
    Some(match spec {
        FillSpec::Solid(c) => FillSpec::Solid(ColorSpec { base: Base::Rgb(0, 0, 0), mods: vec![] }).with_resolved(cx, &c, ph),
        FillSpec::Grad { angle, stops } => FillSpec::Grad {
            angle,
            stops: stops.iter().map(|(p, c)| {
                let r = cx.ch.color(c, ph);
                (*p, ColorSpec { base: Base::Rgb(r.0, r.1, r.2), mods: vec![("alpha".into(), r.3 as f32 / 255.0 * 100000.0)] })
            }).collect(),
        },
        other => other,
    })
}

trait WithResolved {
    fn with_resolved(self, cx: &Cx, c: &ColorSpec, ph: Option<Rgba>) -> FillSpec;
}

impl WithResolved for FillSpec {
    fn with_resolved(self, cx: &Cx, c: &ColorSpec, ph: Option<Rgba>) -> FillSpec {
        let r = cx.ch.color(c, ph);
        FillSpec::Solid(ColorSpec { base: Base::Rgb(r.0, r.1, r.2), mods: vec![("alpha".into(), r.3 as f32 / 255.0 * 100000.0)] })
    }
}

/// The pattern of a shape's outline: `a:ln/a:prstDash`.
fn ln_dash(sp_pr: Option<Node>) -> LineDash {
    match sp_pr.and_then(|p| child(p, "ln")).and_then(|l| child(l, "prstDash")).and_then(|d| d.attribute("val")) {
        Some("sysDot") | Some("dot") => LineDash::Dotted,
        Some("solid") | None => LineDash::Solid,
        Some(_) => LineDash::Dashed,
    }
}

fn shape_line(cx: &Cx, sp_pr: Option<Node>, style: &Style, lph: Option<&PhInfo>, mph: Option<&PhInfo>) -> Option<Stroke> {
    let ln = sp_pr.and_then(|p| child(p, "ln"));
    let explicit = ln.and_then(parse_fill);
    let w_attr = ln.and_then(|l| attr_f(l, "w")).or_else(|| lph.and_then(|p| p.line_w)).or_else(|| mph.and_then(|p| p.line_w));
    let fill = explicit.clone().or_else(|| lph.and_then(|p| p.line.clone())).or_else(|| mph.and_then(|p| p.line.clone()));
    let from_style = || -> Option<(Rgba, f32)> {
        let (idx, clr) = style.ln_ref.clone()?;
        if idx == 0 { return None; }
        let c = clr.map(|c| cx.ch.color(&c, None))?;
        Some((c, cx.ch.theme.line_widths.get(idx - 1).copied().unwrap_or(9525.0)))
    };
    let (color, w) = match fill {
        Some(FillSpec::NoFill) => return None,
        Some(f) => (f.flat(&cx.ch.cx())?, w_attr.unwrap_or(9525.0)),
        None => { let (c, w) = from_style()?; (c, w_attr.unwrap_or(w)) }
    };
    if color.3 == 0 { return None; }
    Some(Stroke { color, width: (w * cx.k).max(1.0) })
}

fn shape_shadow(cx: &Cx, sp_pr: Option<Node>) -> Option<Shadow> {
    let s = child(child(sp_pr?, "effectLst")?, "outerShdw")?;
    let (dist, dir) = (attr_f(s, "dist").unwrap_or(0.0), attr_f(s, "dir").unwrap_or(0.0) / 60000.0);
    let color = first_color(s).map(|c| cx.ch.color(&c, None)).unwrap_or(Rgba(0, 0, 0, 100));
    let a = dir.to_radians();
    Some(Shadow { dx: dist * cx.k * a.cos(), dy: dist * cx.k * a.sin(), blur: attr_f(s, "blurRad").unwrap_or(0.0) * cx.k, spread: 0.0, color })
}

fn flip_path(cmds: Vec<PathCmd>, fh: bool, fv: bool) -> Vec<PathCmd> {
    let fx = |x: f32| if fh { 1.0 - x } else { x };
    let fy = |y: f32| if fv { 1.0 - y } else { y };
    cmds.into_iter().map(|c| match c {
        PathCmd::Move(x, y) => PathCmd::Move(fx(x), fy(y)),
        PathCmd::Line(x, y) => PathCmd::Line(fx(x), fy(y)),
        PathCmd::Curve(a, b, c2, d, e, f) => PathCmd::Curve(fx(a), fy(b), fx(c2), fy(d), fx(e), fy(f)),
        PathCmd::Close => PathCmd::Close,
    }).collect()
}

fn link_of(cx: &mut Cx, click: Node) -> Option<Link> {
    let action = click.attribute("action").unwrap_or("");
    match action {
        "ppaction://hlinkshowjump?jump=nextslide" => return Some(Link::Next),
        "ppaction://hlinkshowjump?jump=previousslide" => return Some(Link::Previous),
        "ppaction://hlinkshowjump?jump=firstslide" => return Some(Link::First),
        "ppaction://hlinkshowjump?jump=lastslide" => return Some(Link::Last),
        _ => {}
    }
    let rid = click.attributes().find(|a| a.name() == "id" && a.namespace().is_some())?.value();
    let rel = cx.rels.get(rid)?;
    if rel.external {
        let u = rel.target.trim();
        let lower = u.to_ascii_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:") { return Some(Link::Url(u.to_string())); }
        cx.report.add("A link that is not http, https or mailto was skipped.");
        return None;
    }
    if rel.kind == "slide" { return Some(Link::Slide(rel.target.clone())); }
    None
}

fn convert_sp(cx: &mut Cx, sp: Node, tf: Tf, out: &mut Vec<Object>) {
    let cnv = child(sp, "nvSpPr").and_then(|n| child(n, "cNvPr"));
    let ph = ph_of(sp);
    let sp_pr = child(sp, "spPr");
    let tx = child(sp, "txBody");
    let has_text = tx.is_some_and(|t| children(t, "p").any(|p| p.descendants().any(|n| n.tag_name().name() == "t" && n.text().is_some_and(|s| !s.trim().is_empty()))));
    // Slide numbers, dates and footers are generated by PowerPoint; keep a footer only if it has text.
    if let Some((t, _)) = &ph {
        if !cx.template && matches!(t.as_str(), "dt" | "sldNum") { return; }
        if !cx.template && t == "ftr" && !has_text { return; }
        if cx.template && matches!(t.as_str(), "dt" | "sldNum" | "ftr") { return; }
    }
    let pk = ph.as_ref().and_then(|(t, _)| kind_of(t));
    if cx.template && ph.is_some() && pk.is_none() { return; }
    let (lph, mph) = resolve_ph(cx, &ph);
    let (lph, mph) = (lph.cloned(), mph.cloned());
    let xfrm = sp_pr.and_then(|p| child(p, "xfrm")).and_then(parse_xfrm).or_else(|| lph.as_ref().and_then(|p| p.xfrm)).or_else(|| mph.as_ref().and_then(|p| p.xfrm));
    let Some(xfrm) = xfrm else { return };
    let xf = tf.apply(&xfrm);
    let style = read_style(sp);
    let fill_spec = shape_fill(cx, sp_pr, &style, lph.as_ref(), mph.as_ref());
    let mut fill = fill_spec.as_ref().and_then(|f| f.flat(&cx.ch.cx()));
    let stroke = shape_line(cx, sp_pr, &style, lph.as_ref(), mph.as_ref());
    let id = cx.next_id();
    let name = cnv.and_then(|c| c.attribute("name")).unwrap_or("Shape").to_string();
    let mut o = Object::new(&id, &name, Kind::Rect, xf.x, xf.y, xf.w, xf.h);
    o.hidden = cnv.is_some_and(|c| c.attribute("hidden") == Some("1"));
    o.alt = cnv.and_then(|c| c.attribute("descr")).unwrap_or("").to_string();
    o.shadow = shape_shadow(cx, sp_pr);
    if let Some(FillSpec::Grad { angle, stops }) = &fill_spec {
        o.gradient = Some(Gradient { angle: *angle, stops: stops.iter().map(|(p, c)| (*p, cx.ch.color(c, None))).collect() });
    }

    // Picture fill: the shape is a picture.
    if let Some(FillSpec::Blip { embed, crop }) = &fill_spec {
        if let Some(asset) = picture_asset(cx, embed, *crop) {
            o.kind = Kind::Image;
            o.asset = Some(asset);
            fill = None;
        }
    }
    let geom_node = sp_pr.and_then(|p| child(p, "prstGeom"));
    let cust = sp_pr.and_then(|p| child(p, "custGeom"));
    let (w, h) = (xf.w.max(1.0), xf.h.max(1.0));
    if matches!(o.kind, Kind::Rect) {
        if let Some(c) = cust {
            let cmds = geom::custom_path(c);
            if !cmds.is_empty() { o.kind = Kind::Path(flip_path(cmds, xf.flip_h, xf.flip_v)); }
        } else if let Some(g) = geom_node {
            let pname = g.attribute("prst").unwrap_or("rect");
            let adj = adj_reader(Some(g));
            match geom::preset(pname, w, h, &adj) {
                Preset::Rect => {}
                Preset::Round(r) => o.radius = r * w.min(h),
                Preset::Ellipse => o.kind = Kind::Ellipse,
                Preset::Line => o.kind = if xf.flip_v != xf.flip_h { Kind::LineUp } else { Kind::Line },
                Preset::Path(cmds) => o.kind = Kind::Path(flip_path(cmds, xf.flip_h, xf.flip_v)),
                Preset::Unknown => cx.report.add(format!("The \"{pname}\" shape is drawn as a rectangle.")),
            }
        }
    }
    let rot = xf.rot.rem_euclid(360.0);
    if rot > 0.05 && rot < 359.95 { o.rotation = rot; }
    o.fill = fill;
    o.stroke = stroke;
    o.dash = ln_dash(sp_pr);

    // Text.
    let tc = TextCtx { family: ph_family(&ph), lph: lph.as_ref(), mph: mph.as_ref(), font_ref: style.font_color.as_ref().map(|c| cx.ch.color(c, None)), force_bold: false, force_color: None };
    apply_text(cx, &mut o, tx, &tc);

    if cx.template {
        // A placeholder in a layout or master: a template slot with the standard prompt.
        if let Some(k) = pk {
            o.placeholder = Some(k);
            o.name = format!("{} placeholder", k.label());
            o.text = k.prompt().into();
            o.rich = None;
            out.push(o);
        } else if o.fill.is_some() || o.stroke.is_some() || matches!(o.kind, Kind::Path(_) | Kind::Image) || !o.text.is_empty() {
            if o.fill.is_none() && o.stroke.is_none() && matches!(o.kind, Kind::Rect) { o.kind = Kind::Text; }
            out.push(o);
        }
        return;
    }
    if let Some(k) = pk {
        o.placeholder = Some(k);
        if o.fill.is_none() && o.stroke.is_none() { o.kind = Kind::Text; }
        out.push(o);
        return;
    }
    let visible = o.fill.is_some() || o.stroke.is_some() || o.shadow.is_some() || matches!(o.kind, Kind::Path(_) | Kind::Image | Kind::Line | Kind::LineUp);
    if !visible && o.text.is_empty() { return; }
    if !visible && matches!(o.kind, Kind::Rect) { o.kind = Kind::Text; }
    // Clickable shapes.
    let text_link = tx.and_then(|t| t.descendants().find(|n| n.is_element() && n.tag_name().name() == "hlinkClick"));
    if let Some(click) = cnv.and_then(|c| child(c, "hlinkClick")) {
        if let Some(l) = link_of(cx, click) { cx.links.push((o.id.clone(), l)); }
    } else if let Some(click) = text_link.filter(|_| !o.rich.as_ref().is_some_and(|r| r.iter().any(|p| p.runs.iter().any(|x| x.link.is_some())))) {
        // A link on some words makes the whole text box clickable here.
        if let Some(l) = link_of(cx, click) {
            cx.links.push((o.id.clone(), l));
            cx.report.add("Links inside text make the whole text box clickable.");
        }
    }
    if o.text_rotation == 0 && tx.is_some() && has_text && o.rich.is_none() && o.text.len() > 4000 { cx.report.add("Very long text boxes may not fit."); }
    out.push(o);
}

fn convert_connector(cx: &mut Cx, c: Node, tf: Tf, out: &mut Vec<Object>) {
    let sp_pr = child(c, "spPr");
    let Some(xfrm) = sp_pr.and_then(|p| child(p, "xfrm")).and_then(parse_xfrm) else { return };
    let xf = tf.apply(&xfrm);
    let style = read_style(c);
    let Some(stroke) = shape_line(cx, sp_pr, &style, None, None) else { return };
    let pname = sp_pr.and_then(|p| child(p, "prstGeom")).and_then(|g| g.attribute("prst")).unwrap_or("line");
    if !matches!(pname, "line" | "straightConnector1") { cx.report.add("Elbow and curved connectors are drawn as straight lines."); }
    let id = cx.next_id();
    let name = child(c, "nvCxnSpPr").and_then(|n| child(n, "cNvPr")).and_then(|n| n.attribute("name")).unwrap_or("Line");
    let kind = if xf.flip_h != xf.flip_v { Kind::LineUp } else { Kind::Line };
    let mut o = Object::new(&id, name, kind, xf.x, xf.y, xf.w, xf.h);
    o.rotation = xf.rot.rem_euclid(360.0);
    o.stroke = Some(stroke);
    o.shadow = shape_shadow(cx, sp_pr);
    out.push(o);
}

/// Store the picture a relationship points at (cropped if asked) and return its asset key.
pub fn picture_asset(cx: &mut Cx, embed: &str, crop: [f32; 4]) -> Option<String> {
    let rel = cx.rels.get(embed)?.clone();
    if rel.external { cx.report.add("A linked (not embedded) picture was skipped."); return None; }
    let ext = rel.target.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => {
            cx.report.add(format!("A .{ext} picture could not be imported (only PNG, JPEG, GIF and SVG are supported)."));
            return None;
        }
    };
    let bytes = cx.pkg.bytes(&rel.target)?;
    let name = rel.target.rsplit('/').next().unwrap_or("image").to_string();
    let cropped = crop.iter().any(|c| *c > 0.001) && matches!(mime, "image/png" | "image/jpeg");
    let key = format!("{}|{:?}", rel.target, crop.map(|c| (c * 1000.0) as i32));
    Some(cx.assets.add(key, &name, mime, || {
        if cropped {
            if let Ok(img) = image::load_from_memory(bytes) {
                let (w, h) = (img.width() as f32, img.height() as f32);
                let (x, y) = ((crop[0] * w) as u32, (crop[1] * h) as u32);
                let (cw, ch) = (((1.0 - crop[0] - crop[2]) * w).max(1.0) as u32, ((1.0 - crop[1] - crop[3]) * h).max(1.0) as u32);
                let sub = img.crop_imm(x, y, cw, ch);
                let mut buf = std::io::Cursor::new(Vec::new());
                let fmt = if mime == "image/png" { image::ImageFormat::Png } else { image::ImageFormat::Jpeg };
                if sub.write_to(&mut buf, fmt).is_ok() { return buf.into_inner(); }
            }
        }
        bytes.clone()
    }))
}

fn media_mime(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "ogg" | "oga" => "audio/ogg",
        _ => return None,
    })
}

fn convert_pic(cx: &mut Cx, pic: Node, tf: Tf, out: &mut Vec<Object>) {
    let nv = child(pic, "nvPicPr");
    let cnv = nv.and_then(|n| child(n, "cNvPr"));
    let ph = ph_of(pic);
    let (lph, mph) = resolve_ph(cx, &ph);
    let sp_pr = child(pic, "spPr");
    let xfrm = sp_pr.and_then(|p| child(p, "xfrm")).and_then(parse_xfrm).or_else(|| lph.and_then(|p| p.xfrm)).or_else(|| mph.and_then(|p| p.xfrm));
    let Some(xfrm) = xfrm else { return };
    let xf = tf.apply(&xfrm);
    let id = cx.next_id();
    let name = cnv.and_then(|c| c.attribute("name")).unwrap_or("Picture").to_string();
    let mut o = Object::new(&id, &name, Kind::Image, xf.x, xf.y, xf.w, xf.h);
    o.rotation = xf.rot.rem_euclid(360.0);
    o.hidden = cnv.is_some_and(|c| c.attribute("hidden") == Some("1"));
    o.alt = cnv.and_then(|c| c.attribute("descr")).unwrap_or("").to_string();
    let style = read_style(pic);
    o.stroke = shape_line(cx, sp_pr, &style, None, None);
    o.shadow = shape_shadow(cx, sp_pr);
    if let Some(prst) = sp_pr.and_then(|p| child(p, "prstGeom")).and_then(|g| g.attribute("prst")).filter(|p| *p != "rect") {
        match PictureShape::from_preset(prst) {
            Some(shape) => o.mask = Some(shape),
            None => cx.report.add("Pictures cropped to some shapes are shown as rectangles."),
        }
    }
    let nvpr = nv.and_then(|n| child(n, "nvPr"));
    let is_video = nvpr.is_some_and(|n| child(n, "videoFile").is_some());
    let is_audio = nvpr.is_some_and(|n| child(n, "audioFile").is_some());
    if is_video || is_audio {
        let media = pic.descendants().filter(|n| n.is_element() && n.tag_name().name() == "media").find_map(|n| n.attributes().find(|a| a.name() == "embed").map(|a| a.value().to_string()));
        let target = media.and_then(|m| cx.rels.get(&m)).filter(|r| !r.external).map(|r| r.target.clone());
        if let Some(t) = target {
            let ext = t.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
            if let (Some(mime), Some(bytes)) = (media_mime(&ext), cx.pkg.bytes(&t)) {
                let fname = t.rsplit('/').next().unwrap_or("media").to_string();
                let key = cx.assets.add(t.clone(), &fname, mime, || bytes.clone());
                o.kind = if mime.starts_with("video") { Kind::Video } else { Kind::Audio };
                o.asset = Some(key);
                out.push(o);
                return;
            }
            cx.report.add(format!("A .{ext} media file could not be imported (use MP4, WebM, MP3, M4A, WAV or OGG); its picture was kept."));
        } else {
            cx.report.add("Linked (not embedded) media was skipped.");
        }
    }
    let blip = child(child(pic, "blipFill").unwrap_or(pic), "blip");
    let Some(embed) = blip.and_then(|b| b.attributes().find(|a| a.name() == "embed")).map(|a| a.value().to_string()) else { return };
    let mut crop = [0.0f32; 4];
    if let Some(r) = child(child(pic, "blipFill").unwrap_or(pic), "srcRect") {
        for (i, k) in ["l", "t", "r", "b"].iter().enumerate() { crop[i] = (attr_f(r, k).unwrap_or(0.0) / 100000.0).clamp(0.0, 0.95); }
    }
    let Some(asset) = picture_asset(cx, &embed, crop) else { return };
    o.asset = Some(asset);
    if let Some(click) = cnv.and_then(|c| child(c, "hlinkClick")) {
        if let Some(l) = link_of(cx, click) { cx.links.push((o.id.clone(), l)); }
    }
    out.push(o);
}

fn convert_frame(cx: &mut Cx, frame: Node, tf: Tf, out: &mut Vec<Object>) {
    let xfrm = child(frame, "xfrm").and_then(parse_xfrm);
    let Some(xfrm) = xfrm else { return };
    let xf = tf.apply(&xfrm);
    let name = child(frame, "nvGraphicFramePr").and_then(|n| child(n, "cNvPr")).and_then(|n| n.attribute("name")).unwrap_or("Content").to_string();
    let data = child(child(frame, "graphic").unwrap_or(frame), "graphicData");
    if let Some(tbl) = data.and_then(|d| child(d, "tbl")) {
        convert_table(cx, tbl, &xf, tf, &name, out);
        return;
    }
    let uri = data.and_then(|d| d.attribute("uri")).unwrap_or("");
    if uri.ends_with("/diagram") {
        // SmartArt keeps a pre-drawn copy of itself (a drawing part of ordinary shapes).
        if let Some(xml) = data.and_then(|d| diagram_drawing(cx, d)) {
            if let Ok(doc) = roxmltree::Document::parse(&xml) {
                if let Some(tree) = doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "spTree") {
                    let before = out.len();
                    convert_children(cx, tree, Tf { sx: tf.sx, sy: tf.sy, tx: xf.x, ty: xf.y }, out);
                    if out.len() > before { return; }
                }
            }
        }
    }
    let what = if uri.ends_with("/chart") { "chart" } else if uri.ends_with("/diagram") { "SmartArt graphic" } else if uri.contains("ole") || child(data.unwrap_or(frame), "oleObj").is_some() { "embedded object" } else { "content" };
    cx.report.add(format!("A {what} could not be imported and was replaced with a grey box."));
    let id = cx.next_id();
    let mut o = Object::new(&id, &name, Kind::Rect, xf.x, xf.y, xf.w, xf.h);
    o.fill = Some(Rgba(235, 237, 240, 255));
    o.text = format!("[{what}]");
    o.style.size = (14.0 * cx.fk).round();
    o.style.align = Align::Center;
    o.style.color = Rgba(110, 118, 130, 255);
    out.push(o);
}

/// The text of a SmartArt graphic's drawing part, found through its data part.
fn diagram_drawing(cx: &Cx, data: Node) -> Option<String> {
    let ids = data.descendants().find(|n| n.is_element() && n.tag_name().name() == "relIds")?;
    let dm = ids.attributes().find(|a| a.name() == "dm")?.value();
    let data_part = &cx.rels.get(dm)?.target;
    let xml = cx.pkg.text(data_part)?;
    let doc = roxmltree::Document::parse(&xml).ok()?;
    let rid = doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "dataModelExt")?.attribute("relId")?.to_string();
    cx.pkg.text(&cx.rels.get(&rid)?.target)
}

fn convert_table(cx: &mut Cx, tbl: Node, xf: &Xfrm, tf: Tf, name: &str, out: &mut Vec<Object>) {
    let cols: Vec<f32> = child(tbl, "tblGrid").map(|g| children(g, "gridCol").map(|c| attr_f(c, "w").unwrap_or(0.0)).collect()).unwrap_or_default();
    let rows: Vec<Node> = children(tbl, "tr").collect();
    if cols.is_empty() || rows.is_empty() { return; }
    let pr = child(tbl, "tblPr");
    let style_id = pr.and_then(|p| child(p, "tableStyleId")).and_then(|s| s.text()).unwrap_or("").trim().to_string();
    let first_row = pr.is_some_and(|p| p.attribute("firstRow") == Some("1"));
    let banded = pr.is_some_and(|p| p.attribute("bandRow") == Some("1"));
    let styled = !style_id.is_empty() && style_id != "{2D5ABB26-0587-4C30-8999-92F81FD0307C}" && style_id != "{5940675A-B579-460E-94D1-54222C63F5DA}";
    if styled { cx.report.add("Table styles are approximated (header colour and banded rows)."); }
    // Cells stay an editable table; merged cells keep their span.
    let table_group = cx.next_id() + "-tbl";
    let accent = |tint: f32| ColorSpec { base: Base::Scheme("accent1".into()), mods: vec![("tint".into(), tint)] };
    let mut y = xf.y;
    let mut covered = vec![vec![false; cols.len()]; rows.len()];
    for (ri, row) in rows.iter().enumerate() {
        let rh = attr_f(*row, "h").unwrap_or(0.0) * tf.sy;
        let mut x = xf.x;
        let mut ci = 0usize;
        for tc in children(*row, "tc") {
            while ci < cols.len() && covered[ri][ci] { x += cols[ci] * tf.sx; ci += 1; }
            if ci >= cols.len() { break; }
            let span = tc.attribute("gridSpan").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1);
            let rspan = tc.attribute("rowSpan").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1).max(1);
            for dr in 0..rspan { for dc in 0..span { if ri + dr < rows.len() && ci + dc < cols.len() && (dr > 0 || dc > 0) { covered[ri + dr][ci + dc] = true; } } }
            let w: f32 = (ci..(ci + span).min(cols.len())).map(|c| cols[c] * tf.sx).sum();
            let h: f32 = rh + (1..rspan).map(|d| rows.get(ri + d).and_then(|r| attr_f(*r, "h")).unwrap_or(0.0) * tf.sy).sum::<f32>();
            let cell_x = x;
            let col_start = ci;
            x += cols[ci..(ci + span).min(cols.len())].iter().map(|c| c * tf.sx).sum::<f32>();
            ci += span;
            if tc.attribute("hMerge") == Some("1") || tc.attribute("vMerge") == Some("1") { continue; }
            let tc_pr = child(tc, "tcPr");
            let header = styled && first_row && ri == 0;
            let mut fill = tc_pr.and_then(parse_fill).and_then(|f| f.flat(&cx.ch.cx()));
            let explicit_fill = tc_pr.and_then(parse_fill).is_some();
            if !explicit_fill && styled {
                fill = Some(if header { cx.ch.color(&ColorSpec { base: Base::Scheme("accent1".into()), mods: vec![] }, None) } else { cx.ch.color(&accent(if banded && (ri - first_row as usize) % 2 == 0 { 40000.0 } else { 20000.0 }), None) });
            }
            // Each side keeps its own line (none when the side is missing or has no fill).
            let mut sides: [Option<Stroke>; 4] = [None; 4];
            for (k, side) in ["lnL", "lnR", "lnT", "lnB"].iter().enumerate() {
                if let Some(ln) = tc_pr.and_then(|p| child(p, side)) {
                    if let Some(c) = parse_fill(ln).and_then(|f| f.flat(&cx.ch.cx())) { sides[k] = Some(Stroke { color: c, width: (attr_f(ln, "w").unwrap_or(12700.0) * cx.k).max(1.0) }); }
                }
            }
            let mixed = sides.iter().any(|s| *s != sides[0]);
            let mut stroke = if mixed { None } else { sides[0] };
            if stroke.is_none() && styled && !mixed { stroke = Some(Stroke { color: Rgba(255, 255, 255, 255), width: (12700.0 * cx.k).max(1.0) }); }
            if stroke.is_none() && !mixed && style_id == "{5940675A-B579-460E-94D1-54222C63F5DA}" { stroke = Some(Stroke { color: Rgba(0, 0, 0, 255), width: (9525.0 * cx.k).max(1.0) }); }
            let id = cx.next_id();
            let mut o = Object::new(&id, &format!("{name} r{}c{}", ri + 1, col_start + 1), Kind::Rect, cell_x, y, w, h);
            o.fill = fill;
            o.stroke = stroke;
            if mixed { o.table_borders = Some(sides); }
            o.group = Some(table_group.clone());
            o.table = Some((ri as u16, col_start as u16));
            if span > 1 || rspan > 1 { o.table_span = Some((rspan as u16, span as u16)); }
            let tc_ctx = TextCtx { family: "other", lph: None, mph: None, font_ref: None, force_bold: header, force_color: if header { Some(Rgba(255, 255, 255, 255)) } else { None } };
            let tx = child(tc, "txBody");
            apply_text(cx, &mut o, tx, &tc_ctx);
            o.margins = [attr_f(tc_pr.unwrap_or(tc), "marL").unwrap_or(91440.0) * cx.k, attr_f(tc_pr.unwrap_or(tc), "marT").unwrap_or(45720.0) * cx.k, attr_f(tc_pr.unwrap_or(tc), "marR").unwrap_or(91440.0) * cx.k, attr_f(tc_pr.unwrap_or(tc), "marB").unwrap_or(45720.0) * cx.k];
            o.style.valign = tc_pr.and_then(|p| p.attribute("anchor")).map(|a| match a { "ctr" => VAlign::Middle, "b" => VAlign::Bottom, _ => VAlign::Top }).unwrap_or(VAlign::Top);
            if o.fill.is_none() && o.stroke.is_none() && o.table_borders.is_none() { o.kind = Kind::Text; }
            out.push(o);
        }
        y += rh;
    }
}

/// Speaker notes of the slide being converted.
pub fn read_notes(cx: &Cx) -> String {
    let Some(rel) = cx.rels.values().find(|r| r.kind == "notesSlide") else { return String::new() };
    let Some(xml) = cx.pkg.text(&rel.target) else { return String::new() };
    let Ok(doc) = roxmltree::Document::parse(&xml) else { return String::new() };
    let mut lines = vec![];
    for sp in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "sp") {
        if ph_of(sp).is_some_and(|(t, _)| t == "body") {
            if let Some(tx) = child(sp, "txBody") {
                for p in children(tx, "p") {
                    lines.push(p.descendants().filter(|n| n.tag_name().name() == "t").filter_map(|n| n.text()).collect::<String>());
                }
            }
        }
    }
    lines.join("\n").trim().to_string()
}

/// Fade time for the slide's transition (only fades are supported).
pub fn read_transition(root: Node, cx: &mut Cx) -> u32 {
    let Some(t) = root.descendants().find(|n| n.is_element() && n.tag_name().name() == "transition") else { return 0 };
    let kind = t.children().find(|c| c.is_element()).map(|c| c.tag_name().name().to_string());
    let ms = t.attributes().find(|a| a.name() == "dur").and_then(|a| a.value().parse::<u32>().ok()).unwrap_or(match t.attribute("spd") { Some("slow") => 1000, Some("fast") => 500, _ => 700 });
    match kind.as_deref() {
        Some("fade") | Some("dissolve") => ms,
        Some(other) => { cx.report.add(format!("The \"{other}\" slide transition is not supported (slides cut instead).")); 0 }
        None => 0,
    }
}

fn tn_list<'a, 'i>(n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    child(n, "childTnLst")
}

fn effect_kind(preset: u32, sub: u32) -> EffectKind {
    match preset {
        2 | 12 => match sub { 1 => EffectKind::SlideUp, 2 => EffectKind::SlideRight, 4 => EffectKind::SlideDown, 8 => EffectKind::SlideLeft, _ => EffectKind::SlideDown },
        22 | 16 | 4 | 3 => EffectKind::Wipe,
        53 | 31 | 17 => EffectKind::Zoom,
        _ => EffectKind::Fade,
    }
}

/// The slide's animations: entrance, exit, emphasis and motion paths. Effects that start by themselves when the
/// slide opens go on the timeline; each click group of the animation pane becomes a numbered click step.
pub fn read_animations(root: Node, cx: &mut Cx, objects: &mut [Object], (slide_w, slide_h): (f32, f32)) {
    let Some(main) = root.descendants().find(|n| n.is_element() && n.tag_name().name() == "cTn" && n.attribute("nodeType") == Some("mainSeq")) else { return };
    let delay_of = |n: Node| -> u32 {
        child(n, "stCondLst").and_then(|l| children(l, "cond").find_map(|c| c.attribute("delay").and_then(|d| d.parse::<u32>().ok()))).unwrap_or(0)
    };
    let mut clicks = 0u32;
    let mut skipped = false;
    let Some(groups) = tn_list(main) else { return };
    for click in children(groups, "par") {
        let Some(ctn) = child(click, "cTn") else { continue };
        // A group that starts by itself when the slide opens is automatic; the others wait for a click.
        let auto = child(ctn, "stCondLst").is_some_and(|l| children(l, "cond").any(|c| c.attribute("evt") == Some("onBegin")));
        if !auto { clicks += 1; }
        let step = if auto { 0 } else { clicks };
        for sub in tn_list(ctn).into_iter().flat_map(|l| children(l, "par")) {
            let Some(sctn) = child(sub, "cTn") else { continue };
            let sub_delay = delay_of(sctn);
            for eff in tn_list(sctn).into_iter().flat_map(|l| children(l, "par")) {
                let Some(ectn) = child(eff, "cTn") else { continue };
                let class = ectn.attribute("presetClass").unwrap_or("");
                if !matches!(class, "entr" | "exit" | "emph" | "path") { if !class.is_empty() { skipped = true; } continue; }
                let preset = ectn.attribute("presetID").and_then(|v| v.parse().ok()).unwrap_or(10);
                let sub_type = ectn.attribute("presetSubtype").and_then(|v| v.parse().ok()).unwrap_or(0);
                let start = sub_delay + delay_of(ectn);
                let dur = ectn.descendants().filter(|n| n.is_element() && n.tag_name().name() == "cTn").filter_map(|n| n.attribute("dur").and_then(|d| d.parse::<u32>().ok())).filter(|d| *d > 1).max().unwrap_or(if preset == 1 { 1 } else { 500 });
                let targets: Vec<String> = ectn.descendants().filter(|n| n.is_element() && n.tag_name().name() == "spTgt").filter_map(|n| n.attribute("spid").map(String::from)).collect();
                let repeat = match ectn.attribute("repeatCount") { Some("indefinite") => 0, Some(v) => v.parse::<u32>().map(|n| (n / 1000).max(1)).unwrap_or(1), None => 1 };
                let motion = if class == "path" { ectn.descendants().find(|n| n.is_element() && n.tag_name().name() == "animMotion").and_then(|m| m.attribute("path")).and_then(|p| parse_motion_path(p, slide_w, slide_h)) } else { None };
                if class == "path" && motion.is_none() { cx.report.add("A motion path could not be read and was skipped."); }
                for t in targets {
                    for oid in cx.sp_ids.get(&t).cloned().unwrap_or_default() {
                        let Some(o) = objects.iter_mut().find(|o| o.id == oid) else { continue };
                        if class == "emph" {
                            o.emphasis = Some(Emphasis { kind: emphasis_kind(preset), delay_ms: start, duration_ms: dur.max(100), repeat });
                            o.click_steps.emphasis = step;
                            continue;
                        }
                        if class == "path" {
                            if let Some(m) = &motion {
                                o.motion = Some(MotionPath { delay_ms: start, duration_ms: dur.max(100), ..m.clone() });
                                o.click_steps.motion = step;
                            }
                            continue;
                        }
                        let e = Entrance { kind: effect_kind(preset, sub_type), delay_ms: start, duration_ms: dur };
                        if class == "entr" {
                            o.entrance = Some(e);
                            o.click_steps.entrance = step;
                        } else if step > 0 {
                            // Leaves when its click comes (after its own delay).
                            o.exit = Some(e);
                            o.click_steps.exit = step;
                        } else {
                            o.exit = Some(Entrance { delay_ms: 0, ..e });
                            o.timing = Some(Timing { start_ms: 0, dur_ms: Some(start + dur) });
                        }
                    }
                }
            }
        }
    }
    if skipped { cx.report.add("Some animations (such as media or verb effects) are not supported."); }
}

/// The closest emphasis effect for a PowerPoint emphasis preset.
fn emphasis_kind(preset: u32) -> EmphasisKind {
    match preset {
        8 => EmphasisKind::Spin,
        6 | 3 => EmphasisKind::Grow,
        32 | 34 => EmphasisKind::Swing,
        9 | 35 | 10 | 5 => EmphasisKind::Flash,
        _ => EmphasisKind::Pulse,
    }
}

/// A PowerPoint motion path ("M 0 0 L 0.2 0.1 C ... E", in fractions of the slide) as points in slide units
/// relative to where the path starts. Curve segments keep their end points and make the path smooth.
fn parse_motion_path(path: &str, w: f32, h: f32) -> Option<MotionPath> {
    let toks: Vec<&str> = path.split_whitespace().collect();
    let mut pts: Vec<(f32, f32)> = vec![];
    let mut smooth = false;
    let mut i = 0;
    let num = |t: Option<&&str>| t.and_then(|v| v.parse::<f32>().ok());
    while i < toks.len() {
        match toks[i] {
            "M" | "L" | "m" | "l" => { pts.push((num(toks.get(i + 1))? * w, num(toks.get(i + 2))? * h)); i += 3; }
            "C" | "c" => { pts.push((num(toks.get(i + 5))? * w, num(toks.get(i + 6))? * h)); smooth = true; i += 7; }
            "E" | "Z" | "z" => break,
            _ => i += 1,
        }
    }
    if pts.len() < 2 { return None; }
    let (ox, oy) = pts[0];
    let points: Vec<(f32, f32)> = pts.into_iter().map(|(x, y)| (x - ox, y - oy)).collect();
    Some(MotionPath { points, delay_ms: 0, duration_ms: 2000, smooth })
}
