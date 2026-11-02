//! Text properties: levels, inheritance and bullets.
use crate::color::*;
use roxmltree::Node;
use brainbend_doc::Align;

#[derive(Clone, Debug, Default)]
pub struct RunProps {
    /// Points.
    pub sz: Option<f32>,
    pub b: Option<bool>,
    pub i: Option<bool>,
    pub u: Option<bool>,
    pub strike: Option<bool>,
    /// Letter spacing in hundredths of a point.
    pub spc: Option<f32>,
    pub color: Option<ColorSpec>,
    pub font: Option<String>,
    /// Percent of the font size the text is raised (negative = lowered).
    pub baseline: Option<f32>,
    /// `cap="all"`.
    pub cap_all: Option<bool>,
    /// Relationship id of a hyperlink on the run.
    pub link_rid: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Bullet {
    None,
    Char(String),
    Auto(String),
}

#[derive(Clone, Debug, Default)]
pub struct ParaProps {
    pub algn: Option<Align>,
    pub bullet: Option<Bullet>,
    /// Line height as a multiple of single spacing.
    pub line: Option<f32>,
    pub def: RunProps,
}

/// Up to nine paragraph levels of properties.
pub type Lst = Vec<ParaProps>;

pub fn empty_lst() -> Lst {
    vec![ParaProps::default(); 9]
}

pub fn merge_run(a: &mut RunProps, b: &RunProps) {
    if a.cap_all.is_none() { a.cap_all = b.cap_all; }
    if a.sz.is_none() { a.sz = b.sz; }
    if a.b.is_none() { a.b = b.b; }
    if a.i.is_none() { a.i = b.i; }
    if a.u.is_none() { a.u = b.u; }
    if a.strike.is_none() { a.strike = b.strike; }
    if a.spc.is_none() { a.spc = b.spc; }
    if a.color.is_none() { a.color = b.color.clone(); }
    if a.font.is_none() { a.font = b.font.clone(); }
    if a.baseline.is_none() { a.baseline = b.baseline; }
}

impl ParaProps {
    /// Fill every unset property from `lower` (a lower-priority source).
    pub fn fill_from(&mut self, lower: &ParaProps) {
        if self.algn.is_none() { self.algn = lower.algn; }
        if self.bullet.is_none() { self.bullet = lower.bullet.clone(); }
        if self.line.is_none() { self.line = lower.line; }
        merge_run(&mut self.def, &lower.def);
    }
}

pub fn parse_align(v: &str) -> Option<Align> {
    match v {
        "l" | "dist" => Some(Align::Left),
        "just" => Some(Align::Justify),
        "ctr" => Some(Align::Center),
        "r" => Some(Align::Right),
        _ => None,
    }
}

/// Resolve theme font references (`+mj-lt`, `+mn-lt`).
pub fn resolve_font(face: &str, theme: &ThemeData) -> Option<String> {
    match face {
        "+mj-lt" => Some(theme.major.clone()),
        "+mn-lt" => Some(theme.minor.clone()),
        f if f.starts_with('+') || f.is_empty() => None,
        f => Some(f.to_string()),
    }
}

pub fn parse_run_props(n: Node, theme: &ThemeData) -> RunProps {
    let mut r = RunProps::default();
    r.sz = attr_f(n, "sz").map(|v| v / 100.0);
    r.b = n.attribute("b").map(|v| v == "1" || v == "true");
    r.i = n.attribute("i").map(|v| v == "1" || v == "true");
    r.u = n.attribute("u").map(|v| v != "none");
    r.strike = n.attribute("strike").map(|v| v != "noStrike");
    r.spc = attr_f(n, "spc");
    r.baseline = attr_f(n, "baseline").map(|v| v / 1000.0);
    r.cap_all = n.attribute("cap").map(|v| v == "all");
    r.link_rid = child(n, "hlinkClick").and_then(|c| c.attributes().find(|a| a.name() == "id" && a.namespace().is_some()).map(|a| a.value().to_string()));
    if let Some(f) = parse_fill(n) {
        if let FillSpec::Solid(c) = f { r.color = Some(c); }
    }
    r.font = child(n, "latin").and_then(|l| l.attribute("typeface")).and_then(|f| resolve_font(f, theme));
    r
}

/// Parse `lvlNpPr` / `pPr`: alignment, bullet and the default run properties.
pub fn parse_para_props(n: Node, theme: &ThemeData) -> ParaProps {
    let mut p = ParaProps::default();
    p.algn = n.attribute("algn").and_then(parse_align);
    let bu_font = child(n, "buFont").and_then(|f| f.attribute("typeface")).unwrap_or("");
    for c in n.children().filter(|c| c.is_element()) {
        match c.tag_name().name() {
            "buNone" => p.bullet = Some(Bullet::None),
            "buChar" => p.bullet = Some(Bullet::Char(bullet_char(c.attribute("char").unwrap_or("•"), bu_font))),
            "buAutoNum" => p.bullet = Some(Bullet::Auto(c.attribute("type").unwrap_or("arabicPeriod").to_string())),
            "defRPr" => p.def = parse_run_props(c, theme),
            "lnSpc" => p.line = child(c, "spcPct").and_then(|v| attr_f(v, "val")).map(|v| v / 100000.0),
            _ => {}
        }
    }
    p
}

/// Parse an `a:lstStyle` (or `titleStyle`/`bodyStyle`/`otherStyle`/`defaultTextStyle`) element.
pub fn parse_lst(n: Option<Node>, theme: &ThemeData) -> Lst {
    let mut l = empty_lst();
    let Some(n) = n else { return l };
    for c in n.children().filter(|c| c.is_element()) {
        let name = c.tag_name().name();
        if let Some(lvl) = name.strip_prefix("lvl").and_then(|r| r.strip_suffix("pPr")).and_then(|d| d.parse::<usize>().ok()) {
            if (1..=9).contains(&lvl) { l[lvl - 1] = parse_para_props(c, theme); }
        }
    }
    l
}

/// The effective properties for level `lvl` given the sources in priority order.
pub fn effective(own: ParaProps, chain: &[&Lst], lvl: usize) -> ParaProps {
    let mut p = own;
    for l in chain { p.fill_from(&l[lvl.min(8)]); }
    p
}

/// "1.", "a)", "i." ... for the `n`th item (1-based) of an auto-numbered list.
pub fn auto_number(kind: &str, n: usize) -> String {
    let alpha = |upper: bool| {
        let mut s = String::new();
        let mut v = n;
        while v > 0 { v -= 1; s.insert(0, (b'a' + (v % 26) as u8) as char); v /= 26; }
        if upper { s.to_uppercase() } else { s }
    };
    let roman = |upper: bool| {
        let (mut v, mut s) = (n, String::new());
        for (val, sym) in [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")] {
            while v >= val { s.push_str(sym); v -= val; }
        }
        if upper { s.to_uppercase() } else { s }
    };
    match kind {
        "alphaLcParenR" => format!("{})", alpha(false)),
        "alphaUcParenR" => format!("{})", alpha(true)),
        "alphaLcPeriod" => format!("{}.", alpha(false)),
        "alphaUcPeriod" => format!("{}.", alpha(true)),
        "romanLcPeriod" => format!("{}.", roman(false)),
        "romanUcPeriod" => format!("{}.", roman(true)),
        "arabicParenR" => format!("{n})"),
        _ => format!("{n}."),
    }
}

/// Bullets in symbol fonts (Wingdings, Symbol) are stored as plain letters; map the common ones to Unicode.
fn bullet_char(ch: &str, font: &str) -> String {
    let f = font.to_ascii_lowercase();
    let mapped = if f.starts_with("wingdings") {
        match ch {
            "§" | "p" => "▪",
            "Ø" => "➢",
            "v" => "❖",
            "ü" => "✓",
            "û" => "✗",
            "l" => "●",
            "n" => "■",
            "q" => "❑",
            "o" => "□",
            "u" => "◆",
            "à" => "➔",
            "è" => "➔",
            "¡" => "○",
            "J" => "☺",
            "x" => "✘",
            _ => "•",
        }
    } else if f == "symbol" {
        match ch {
            "·" => "•",
            "§" => "♣",
            "Ø" => "⇒",
            _ => "•",
        }
    } else {
        return ch.to_string();
    };
    mapped.to_string()
}
