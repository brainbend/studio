//! Equations: LaTeX typeset into vector outlines.
//!
//! `latex-rust` (pure Rust, STIX Two Math) lays the maths out and writes SVG whose paths are the glyphs and rules; those
//! paths become an ordinary freeform shape, so the editor, the player, print and the .pptx export all draw an equation
//! without knowing what it is. `pulldown-latex` writes the MathML that screen readers get.
use super::*;
use latex_rust::{latex_to_svg, MathFont, SvgOptions};

/// A typeset equation.
pub struct Rendered {
    /// The outlines in 0..1 of the equation's own box (y down).
    pub cmds: Vec<PathCmd>,
    /// Width and height in points at the size it was set (12 pt text).
    pub width_pt: f32,
    pub height_pt: f32,
    pub mathml: String,
}

/// Commands the typesetter lacks, rewritten into ones it has. (`\bmod`, `\pmod`, `\substack` and `\stackrel` are
/// common in coursework.) The MathML for screen readers is made from the author's own text, not this rewrite.
pub fn shim(latex: &str) -> String {
    let mut s = latex.to_string();
    // x \bmod n  ->  x \;\mathrm{mod}\; n
    s = s.replace("\\bmod", "\\;\\mathrm{mod}\\;");
    // \pmod{n}  ->  \ (\mathrm{mod}\ n)   (with the usual space before it)
    while let Some(i) = s.find("\\pmod{") {
        let open = i + "\\pmod".len();
        let Some(end) = matching_brace(&s, open) else { break };
        let arg = s[open + 1..end].to_string();
        s.replace_range(i..=end, &format!("\\;\\left(\\mathrm{{mod}}\\ {arg}\\right)"));
    }
    s = s.replace("\\stackrel", "\\overset");
    // \substack{a \\ b}  ->  a small column of lines
    while let Some(i) = s.find("\\substack{") {
        let open = i + "\\substack".len();
        let Some(end) = matching_brace(&s, open) else { break };
        let body = s[open + 1..end].to_string();
        s.replace_range(i..=end, &format!("\\begin{{array}}{{c}}{body}\\end{{array}}"));
    }
    // \vec{B}: the typesetter stretches the arrow over a wide box and offsets it; the font's combining arrow sits over
    // the letter like TeX's. A longer argument (\vec{AB}) keeps the stretched arrow, which suits it.
    let mut from = 0;
    while let Some(i) = s[from..].find("\\vec{").map(|i| i + from) {
        let open = i + "\\vec".len();
        let Some(end) = matching_brace(&s, open) else { break };
        let arg = s[open + 1..end].trim().to_string();
        let one_letter = arg.chars().count() == 1 && !arg.starts_with('\\');
        let one_command = arg.starts_with('\\') && arg[1..].chars().all(|c| c.is_ascii_alphabetic()) && arg.len() > 1;
        if one_letter || one_command {
            let gap = if s[..i].ends_with(|c: char| c.is_ascii_alphabetic()) { " " } else { "" };
            s.replace_range(i..=end, &format!("{gap}{arg}{}", '\u{20D7}'));
            from = i;
        } else {
            from = end;
        }
    }
    // \vec F  (no braces): the next letter is the argument
    let mut from = 0;
    while let Some(i) = s[from..].find("\\vec").map(|i| i + from) {
        let rest = &s[i + 4..];
        let trimmed = rest.trim_start();
        match trimmed.chars().next() {
            Some(c) if c.is_ascii_alphabetic() && rest.len() > trimmed.len() => {
                let end = i + 4 + (rest.len() - trimmed.len()) + c.len_utf8();
                let gap = if s[..i].ends_with(|c: char| c.is_ascii_alphabetic()) { " " } else { "" };
                s.replace_range(i..end, &format!("{gap}{c}{}", '\u{20D7}'));
                from = i;
            }
            _ => from = i + 4,
        }
    }
    // \ce{H2O}: formulas with plain digits become subscripts, charges stay as written.
    while let Some(i) = s.find("\\ce{") {
        let open = i + "\\ce".len();
        let Some(end) = matching_brace(&s, open) else { break };
        let body = s[open + 1..end].to_string();
        s.replace_range(i..=end, &chemistry(&body));
    }
    s
}

/// The index of the `}` that closes the `{` at `open`.
fn matching_brace(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices().skip_while(|(i, _)| *i < open) {
        match c { '{' => depth += 1, '}' => { depth -= 1; if depth == 0 { return Some(i); } } _ => {} }
    }
    None
}

/// A small subset of mhchem: element symbols stay upright, digits after a letter or `)` are subscripts, `->`, `<-`, `<->` and `<=>` are arrows,
/// `^+`, `^2-` and the like are charges, and a number at the start of a term (a coefficient) stays at full size.
fn chemistry(src: &str) -> String {
    let mut out = String::from("\\mathrm{");
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut prev_alpha = false;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '-' if chars.get(i + 1) == Some(&'>') => { out.push_str("}\\rightarrow\\mathrm{"); i += 2; prev_alpha = false; continue; }
            '<' if chars[i..].starts_with(&['<', '-', '>']) => { out.push_str("}\\leftrightarrow\\mathrm{"); i += 3; prev_alpha = false; continue; }
            '<' if chars[i..].starts_with(&['<', '=', '>']) => { out.push_str("}\\rightleftharpoons\\mathrm{"); i += 3; prev_alpha = false; continue; }
            '<' if chars[i..].starts_with(&['<', '-']) => { out.push_str("}\\leftarrow\\mathrm{"); i += 2; prev_alpha = false; continue; }
            '+' => { out.push_str("}+\\mathrm{"); prev_alpha = false; }
            '^' => {
                // A charge: ^+, ^-, ^2+, ^{2-}
                let mut j = i + 1;
                let mut charge = String::new();
                if chars.get(j) == Some(&'{') { j += 1; while j < chars.len() && chars[j] != '}' { charge.push(chars[j]); j += 1; } j += 1; }
                else { while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '+' || chars[j] == '-') { charge.push(chars[j]); j += 1; } }
                out.push_str(&format!("^{{{}}}", charge.replace('-', "\\text{-}")));
                i = j; prev_alpha = false; continue;
            }
            d if d.is_ascii_digit() && (prev_alpha || (i > 0 && chars[i - 1] == ')')) => {
                let mut j = i;
                let mut n = String::new();
                while j < chars.len() && chars[j].is_ascii_digit() { n.push(chars[j]); j += 1; }
                out.push_str(&format!("_{{{n}}}"));
                i = j; prev_alpha = false; continue;
            }
            ' ' => out.push_str("\\ "),
            _ => out.push(c),
        }
        prev_alpha = c.is_ascii_alphabetic() || c == ')';
        i += 1;
    }
    out.push('}');
    out.replace("\\mathrm{}", "")
}

/// Typeset `latex`. `display` is display style (stacked limits, larger operators); otherwise inline style.
pub fn render(latex: &str, display: bool) -> Result<Rendered, String> {
    let latex = latex.trim();
    if latex.is_empty() { return Err("Type some LaTeX.".into()); }
    let font = MathFont::stix_two_math().map_err(|e| format!("The maths font could not be loaded: {e:?}"))?;
    let mut opts = SvgOptions::new();
    opts.display = display;
    let svg = latex_to_svg(&shim(latex), &font, &opts).map_err(|e| explain(&format!("{e:?}")))?;
    let (cmds, width_pt, height_pt) = svg_to_path(&svg).ok_or("The typeset result could not be read.")?;
    Ok(Rendered { cmds, width_pt, height_pt, mathml: mathml(latex, display) })
}

/// A readable message for the typesetter's error (`Parse(Unknown("\\SI"))` and the like).
fn explain(e: &str) -> String {
    if let Some(i) = e.find("Unknown(\"") {
        let name = e[i + 9..].split('"').next().unwrap_or("").replace("\\\\", "\\");
        return format!("{name} is not supported here. Check the spelling, or use a different command.");
    }
    format!("This LaTeX could not be typeset ({e}).")
}

/// MathML for screen readers, from the author's own LaTeX; empty if it cannot be made.
pub fn mathml(latex: &str, display: bool) -> String {
    use pulldown_latex::{config::{DisplayMode, RenderConfig}, push_mathml, Parser, Storage};
    let storage = Storage::new();
    let parser = Parser::new(latex.trim(), &storage);
    let mut out = String::new();
    let cfg = RenderConfig { display_mode: if display { DisplayMode::Block } else { DisplayMode::Inline }, ..Default::default() };
    match push_mathml(&mut out, parser, cfg) { Ok(()) => out, Err(_) => String::new() }
}

/// The glyph paths of the typesetter's SVG as one path in 0..1 coordinates, with the size in points.
fn svg_to_path(svg: &str) -> Option<(Vec<PathCmd>, f32, f32)> {
    let attr = |tag: &str, name: &str| -> Option<String> {
        let at = tag.find(&format!(" {name}=\""))? + name.len() + 3;
        Some(tag[at..].split('"').next()?.to_string())
    };
    let head = &svg[svg.find("<svg")?..];
    let head = &head[..head.find('>')?];
    let pt = |v: String| v.trim_end_matches("pt").parse::<f32>().ok();
    let (width_pt, height_pt) = (pt(attr(head, "width")?)?, pt(attr(head, "height")?)?);
    let vb: Vec<f32> = attr(head, "viewBox")?.split_whitespace().filter_map(|v| v.parse().ok()).collect();
    let (vw, vh) = (*vb.get(2)?, *vb.get(3)?);
    if vw <= 0.0 || vh <= 0.0 { return None; }
    let mut cmds = vec![];
    for chunk in svg.split("<path ").skip(1) {
        let tag = &chunk[..chunk.find("/>").unwrap_or(chunk.len())];
        let (d, transform) = (attr(&format!(" {tag}"), "d")?, attr(&format!(" {tag}"), "transform").unwrap_or_default());
        // transform="translate(tx ty) scale(sx sy)"
        let nums = |after: &str| -> Vec<f32> {
            transform.find(after).map(|i| transform[i + after.len()..].split(')').next().unwrap_or("").split_whitespace().filter_map(|v| v.parse().ok()).collect()).unwrap_or_default()
        };
        let (t, sc) = (nums("translate("), nums("scale("));
        let (tx, ty) = (t.first().copied().unwrap_or(0.0), t.get(1).copied().unwrap_or(0.0));
        let (sx, sy) = (sc.first().copied().unwrap_or(1.0), sc.get(1).copied().unwrap_or(sc.first().copied().unwrap_or(1.0)));
        let at = |x: f32, y: f32| ((tx + sx * x) / vw, (ty + sy * y) / vh);
        let mut tokens = d.split_whitespace().peekable();
        while let Some(op) = tokens.next() {
            let mut n = |k: usize| -> Option<Vec<f32>> { (0..k).map(|_| tokens.next()?.parse().ok()).collect() };
            match op {
                "M" => { let v = n(2)?; let p = at(v[0], v[1]); cmds.push(PathCmd::Move(p.0, p.1)); }
                "L" => { let v = n(2)?; let p = at(v[0], v[1]); cmds.push(PathCmd::Line(p.0, p.1)); }
                "C" => { let v = n(6)?; let (a, b, c) = (at(v[0], v[1]), at(v[2], v[3]), at(v[4], v[5])); cmds.push(PathCmd::Curve(a.0, a.1, b.0, b.1, c.0, c.1)); }
                "Z" => cmds.push(PathCmd::Close),
                _ => return None,
            }
        }
    }
    // Rules (fraction bars, the bar over a root, brace rules) are rectangles. They are wound the way the glyphs' outlines
    // are, so where a rule touches a glyph (a root sign) the two add up rather than cancel.
    let rects: Vec<(f32, f32, f32, f32)> = svg.split("<rect ").skip(1).filter_map(|chunk| {
        let tag = format!(" {}", &chunk[..chunk.find("/>").unwrap_or(chunk.len())]);
        let v = |n: &str| attr(&tag, n)?.parse::<f32>().ok();
        let (x, y, w, h) = (v("x")?, v("y")?, v("width")?, v("height")?);
        Some((x / vw, y / vh, (x + w) / vw, (y + h) / vh))
    }).collect();
    let mut figures: Vec<Vec<(f32, f32)>> = vec![];
    for c in &cmds {
        match *c {
            PathCmd::Move(x, y) => figures.push(vec![(x, y)]),
            PathCmd::Line(x, y) | PathCmd::Curve(_, _, _, _, x, y) => { if let Some(f) = figures.last_mut() { f.push((x, y)); } }
            PathCmd::Close => {}
        }
    }
    let area = |f: &[(f32, f32)]| (0..f.len()).map(|i| { let (a, b) = (f[i], f[(i + 1) % f.len()]); a.0 * b.1 - b.0 * a.1 }).sum::<f32>() / 2.0;
    let biggest = figures.iter().map(|f| area(f)).max_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap_or(1.0);
    for (x0, y0, x1, y1) in rects {
        let pts = if biggest >= 0.0 { [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] } else { [(x0, y0), (x0, y1), (x1, y1), (x1, y0)] };
        cmds.push(PathCmd::Move(pts[0].0, pts[0].1));
        for p in &pts[1..] { cmds.push(PathCmd::Line(p.0, p.1)); }
        cmds.push(PathCmd::Close);
    }
    (!cmds.is_empty()).then_some((cmds, width_pt, height_pt))
}

/// The Equation window: LaTeX typed (or built from the palette) with a live preview.
pub struct Dialog {
    /// The equation being edited, or `None` for a new one.
    pub target: Option<(usize, usize)>,
    pub latex: String,
    pub display: bool,
    cache: Option<(String, bool, Result<Rendered, String>)>,
}

const STRUCTURES: &[(&str, &str)] = &[
    ("a/b", r"\frac{a}{b}"), ("√", r"\sqrt{x}"), ("ⁿ√", r"\sqrt[n]{x}"), ("xⁿ", "^{n}"), ("xₙ", "_{n}"),
    ("∑", r"\sum_{i=1}^{n}"), ("∏", r"\prod_{i=1}^{n}"), ("∫", r"\int_{a}^{b}"), ("oint", r"\oint"), ("lim", r"\lim_{x \to 0}"),
    ("( )", r"\left( \right)"), ("[ ]", r"\left[ \right]"), ("∂/∂x", r"\frac{\partial f}{\partial x}"),
    ("matrix", r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}"), ("cases", r"\begin{cases} x & x \ge 0 \\ -x & x < 0 \end{cases}"),
    ("aligned", r"\begin{aligned} a &= b \\ c &= d \end{aligned}"),
    ("vec", r"\vec{v}"), ("hat", r"\hat{x}"), ("bar", r"\bar{x}"), ("dot", r"\dot{x}"), ("text", r"\text{words}"),
];
const GREEK: &[(&str, &str)] = &[
    ("α", r"\alpha"), ("β", r"\beta"), ("γ", r"\gamma"), ("δ", r"\delta"), ("ε", r"\varepsilon"), ("θ", r"\theta"), ("λ", r"\lambda"), ("μ", r"\mu"),
    ("π", r"\pi"), ("ρ", r"\rho"), ("σ", r"\sigma"), ("τ", r"\tau"), ("φ", r"\varphi"), ("ω", r"\omega"),
    ("Γ", r"\Gamma"), ("Δ", r"\Delta"), ("Θ", r"\Theta"), ("Λ", r"\Lambda"), ("Σ", r"\Sigma"), ("Φ", r"\Phi"), ("Ψ", r"\Psi"), ("Ω", r"\Omega"),
];
const SYMBOLS: &[(&str, &str)] = &[
    ("±", r"\pm"), ("×", r"\times"), ("÷", r"\div"), ("·", r"\cdot"), ("≤", r"\le"), ("≥", r"\ge"), ("≠", r"\ne"), ("≈", r"\approx"),
    ("≡", r"\equiv"), ("∝", r"\propto"), ("∞", r"\infty"), ("→", r"\to"), ("⇒", r"\Rightarrow"), ("⇔", r"\Leftrightarrow"),
    ("∈", r"\in"), ("⊂", r"\subset"), ("∪", r"\cup"), ("∩", r"\cap"), ("∀", r"\forall"), ("∃", r"\exists"), ("∇", r"\nabla"),
    ("∂", r"\partial"), ("ħ", r"\hbar"), ("bb R", r"\mathbb{R}"), ("bb Z", r"\mathbb{Z}"), ("bb N", r"\mathbb{N}"),
];

impl App {
    /// Open the Equation window: for a new equation, or to change the equation `target`.
    pub(super) fn open_equation_dialog(&mut self, target: Option<(usize, usize)>) {
        let (latex, display) = target
            .and_then(|(li, oi)| self.project.as_ref()?.slides.get(self.slide)?.layers.get(li)?.objects.get(oi)?.equation.as_ref().map(|e| (e.latex.clone(), e.display)))
            .unwrap_or_default();
        self.eq_dialog = Some(Dialog { target, latex, display: display || target.is_none(), cache: None });
    }

    /// A new equation shape in the theme's text color at the default size.
    pub(super) fn insert_equation(&mut self, latex: &str, display: bool) -> Result<(), String> {
        let r = render(latex, display)?;
        let color = self.project.as_ref().and_then(|p| p.theme.colors.first().copied()).unwrap_or(Rgba(34, 34, 34, 255));
        let mut o = blank_object(self.new_id("obj"), "Equation", Kind::Path(r.cmds), 0.0, 0.0, r.width_pt * DEFAULT_SCALE, r.height_pt * DEFAULT_SCALE);
        o.fill = Some(color);
        o.equation = Some(EquationInfo { latex: latex.trim().to_string(), display, mathml: r.mathml, width_pt: r.width_pt });
        self.insert(o);
        Ok(())
    }

    /// Typeset `latex` again into the equation `(li, oi)`, keeping its color and its scale (so a resized equation stays
    /// the size it was given).
    pub(super) fn update_equation(&mut self, (li, oi): (usize, usize), latex: &str, display: bool) -> Result<(), String> {
        let r = render(latex, display)?;
        self.snapshot();
        let slide = self.slide;
        let Some(o) = self.project.as_mut().and_then(|p| p.slides.get_mut(slide)).and_then(|s| s.layers.get_mut(li)).and_then(|l| l.objects.get_mut(oi)) else { return Err("The equation is gone.".into()) };
        let scale = o.equation.as_ref().filter(|e| e.width_pt > 0.0).map(|e| o.w / e.width_pt).unwrap_or(DEFAULT_SCALE);
        o.w = r.width_pt * scale;
        o.h = r.height_pt * scale;
        o.kind = Kind::Path(r.cmds);
        o.equation = Some(EquationInfo { latex: latex.trim().to_string(), display, mathml: r.mathml, width_pt: r.width_pt });
        self.dirty = true;
        Ok(())
    }

    /// The Equation window, while it is open.
    pub(super) fn equation_window(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.eq_dialog.take() else { return };
        if self.project.is_none() { return; }
        let (mut done, mut cancel) = (false, false);
        let id = egui::Id::new("eq-latex");
        let mut snippet: Option<&str> = None;
        let mut open = true;
        crate::dialog_window(ctx, "Equation").id(egui::Id::new("equation-dialog")).open(&mut open).collapsible(false).resizable(false)
            .default_pos(ctx.content_rect().center() - egui::vec2(290.0, 250.0)).show(ctx, |ui| {
            ui.set_width(560.0);
            ui.add(egui::TextEdit::multiline(&mut dlg.latex).id(id).font(egui::TextStyle::Monospace).desired_rows(4).desired_width(f32::INFINITY).hint_text(r"LaTeX, for example  \frac{a}{b}  or  \int_0^1 x^2\,dx"));
            ui.checkbox(&mut dlg.display, "Display style").on_hover_text("Large operators and limits above and below, as in a displayed equation; off gives the compact in-line style");
            let fam = font_family("Open Sans");
            for (name, items) in [("Structures", STRUCTURES), ("Greek", GREEK), ("Symbols", SYMBOLS)] {
                ui.label(egui::RichText::new(name).weak().size(12.0));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                    for (label, code) in items {
                        let b = ui.add(egui::Button::new(egui::RichText::new(*label).family(fam.clone()).size(15.0)).min_size(egui::vec2(30.0, 26.0)));
                        b.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, *code));
                        if b.on_hover_text(*code).clicked() { snippet = Some(*code); }
                    }
                });
            }
            ui.add_space(4.0);
            // The preview: the equation as it will be typeset, or what is wrong with the LaTeX.
            if dlg.cache.as_ref().is_none_or(|(l, d, _)| *l != dlg.latex || *d != dlg.display) {
                dlg.cache = Some((dlg.latex.clone(), dlg.display, render(&dlg.latex, dlg.display)));
            }
            let (rect, _) = ui.allocate_exact_size(egui::vec2(560.0, 150.0), egui::Sense::hover());
            ui.painter().rect(rect, 6.0, egui::Color32::WHITE, egui::Stroke::new(1.0, egui::Color32::from_gray(150)), egui::StrokeKind::Inside);
            let mut valid = false;
            match dlg.cache.as_ref().map(|(_, _, r)| r) {
                Some(Ok(r)) => {
                    valid = true;
                    let room = rect.shrink2(egui::vec2(16.0, 14.0));
                    let k = (room.width() / r.width_pt).min(room.height() / r.height_pt).min(5.0);
                    let at = egui::Rect::from_center_size(rect.center(), egui::vec2(r.width_pt, r.height_pt) * k);
                    fill_figures(ui.painter(), &path_polygons(&r.cmds, at), &|p| p, &|_| egui::Color32::BLACK);
                }
                Some(Err(e)) if !dlg.latex.trim().is_empty() => { ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, e, egui::FontId::proportional(14.0), egui::Color32::from_rgb(190, 40, 40)); }
                _ => { ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "The equation shows here", egui::FontId::proportional(14.0), egui::Color32::from_gray(150)); }
            }
            ui.horizontal(|ui| {
                let label = if dlg.target.is_some() { "Update" } else { "Insert" };
                if ui.add_enabled(valid, egui::Button::new(label)).clicked() { done = true; }
                if ui.button("Cancel").clicked() { cancel = true; }
                ui.weak("Screen readers get the equation as MathML.");
            });
        });
        // A palette button puts its LaTeX at the cursor (or replaces what is chosen).
        if let Some(code) = snippet {
            let chars: Vec<char> = dlg.latex.chars().collect();
            let (a, b) = egui::TextEdit::load_state(ctx, id).and_then(|st| st.cursor.char_range())
                .map(|r| { let (p, q) = (r.primary.index.0, r.secondary.index.0); (p.min(q).min(chars.len()), p.max(q).min(chars.len())) })
                .unwrap_or((chars.len(), chars.len()));
            let sep = if code.starts_with('\\') && a > 0 && chars[a - 1].is_ascii_alphabetic() { " " } else { "" };
            dlg.latex = chars[..a].iter().collect::<String>() + sep + code + &chars[b..].iter().collect::<String>();
            let at = a + sep.chars().count() + code.chars().count();
            if let Some(mut st) = egui::TextEdit::load_state(ctx, id) {
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(at))));
                st.store(ctx, id);
            }
            ctx.memory_mut(|m| m.request_focus(id));
        }
        if done {
            let result = match dlg.target { Some(t) => self.update_equation(t, &dlg.latex, dlg.display), None => self.insert_equation(&dlg.latex, dlg.display) };
            if let Err(e) = result { self.status = e; } else { return; }
        }
        if open && !cancel { self.eq_dialog = Some(dlg); }
    }
}

/// Points per slide pixel of an equation set at the default size: 12 pt text becomes 48 px, near the slide's text.
pub const DEFAULT_SCALE: f32 = 4.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// Equations of the kind coursework uses; each must typeset (the marker is the expression's first command).
    const SAMPLES: &[&str] = &[
        r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}",
        r"\int_0^\infty e^{-x^2}\,dx=\frac{\sqrt{\pi}}{2}",
        r"\sum_{n=1}^{\infty}\frac{1}{n^2}=\frac{\pi^2}{6}",
        r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
        r"\begin{aligned} \nabla\cdot\mathbf{E} &= \frac{\rho}{\varepsilon_0} \\ \nabla\times\mathbf{B} &= \mu_0\mathbf{J} \end{aligned}",
        r"\lim_{x\to 0}\frac{\sin x}{x}=1",
        r"f(x)=\begin{cases} x^2 & x\ge 0 \\ -x & x<0\end{cases}",
        r"\mathrm{CO_2}+\mathrm{H_2O}\rightarrow\mathrm{H_2CO_3}",
        r"\vec{F}=m\vec{a},\ \hat{x},\ \bar{x},\ \dot{x}",
        r"\binom{n}{k}=\frac{n!}{k!(n-k)!}",
        r"\oint_C \vec F\cdot d\vec r = \iint_S (\nabla\times\vec F)\cdot d\vec S",
        r"\det A=\begin{vmatrix} a&b\\c&d\end{vmatrix}=ad-bc",
        r"\left(\frac{a}{b}\right)^{2} \overset{?}{=} \sqrt[3]{x}",
        r"\forall x\in\mathbb{R}\;\exists y\in\mathcal{L}:\ x<y",
        r"A\xrightarrow{f}B,\ \underbrace{a+b+c}_{n}",
        r"\boxed{E=mc^2}",
        r"\text{if } x>0 \text{ then } \alpha\le\beta",
        r"\hbar\omega=E_n-E_m,\quad \langle\psi|\hat H|\psi\rangle",
        r"\begin{align} a&=b+c\\ d&=e \end{align}",
        r"a \bmod n,\ x \equiv y \pmod{n}",
        r"\sum_{\substack{i<j\\ j<n}} a_{ij}",
        r"\stackrel{def}{=}",
        r"\ce{H2O}+\ce{CO2}->\ce{H2CO3}",
        r"\ce{2H2 + O2 -> 2H2O}",
    ];

    #[test]
    fn coursework_equations_typeset_and_have_mathml() {
        for src in SAMPLES {
            for display in [true, false] {
                let r = render(src, display).unwrap_or_else(|e| panic!("{src}: {e}"));
                assert!(!r.cmds.is_empty() && r.width_pt > 1.0 && r.height_pt > 1.0, "{src}");
                assert!(r.mathml.starts_with("<math"), "{src}: MathML for screen readers");
                // Every coordinate is in the equation's own box (curve handles may poke out a little).
                for c in &r.cmds {
                    let pts: Vec<f32> = match *c { PathCmd::Move(x, y) | PathCmd::Line(x, y) => vec![x, y], PathCmd::Curve(a, b, c, d, e, f) => vec![a, b, c, d, e, f], PathCmd::Close => vec![] };
                    assert!(pts.iter().all(|v| (-0.1..=1.1).contains(v)), "{src}: {pts:?}");
                }
            }
        }
    }

    #[test]
    fn rules_are_drawn_fraction_bars_and_the_bar_over_a_root() {
        // A rule is a flat rectangle: four points, much wider than it is tall.
        let bars = |latex: &str| -> usize {
            let r = render(latex, true).unwrap();
            let mut figs: Vec<Vec<(f32, f32)>> = vec![];
            for c in &r.cmds {
                match *c { PathCmd::Move(x, y) => figs.push(vec![(x, y)]), PathCmd::Line(x, y) | PathCmd::Curve(_, _, _, _, x, y) => figs.last_mut().unwrap().push((x, y)), PathCmd::Close => {} }
            }
            figs.iter().filter(|f| {
                let (w, h) = (f.iter().map(|p| p.0).fold(f32::MIN, f32::max) - f.iter().map(|p| p.0).fold(f32::MAX, f32::min), f.iter().map(|p| p.1).fold(f32::MIN, f32::max) - f.iter().map(|p| p.1).fold(f32::MAX, f32::min));
                f.len() == 4 && w * r.width_pt > h * r.height_pt * 6.0
            }).count()
        };
        assert!(bars(r"\frac{1}{2}") >= 1, "a fraction has its bar");
        assert!(bars(r"\sqrt{x}") >= 1, "a root has the bar over it");
        assert!(bars(r"\frac{a}{b}+\frac{c}{d}") >= 2);
    }

    #[test]
    fn display_style_stacks_limits_and_is_taller_than_inline() {
        let (d, i) = (render(r"\sum_{n=1}^{\infty}\frac{1}{n^2}", true).unwrap(), render(r"\sum_{n=1}^{\infty}\frac{1}{n^2}", false).unwrap());
        assert!(d.height_pt > i.height_pt, "display {} vs inline {}", d.height_pt, i.height_pt);
    }

    #[test]
    fn unsupported_commands_say_which() {
        let e = render(r"\SI{5}{\meter}", true).err().unwrap();
        assert!(e.contains("\\SI") && e.contains("not supported"), "{e}");
        assert!(render("   ", true).is_err());
        assert!(render(r"\frac{1}{", true).is_err(), "an unfinished group is an error, not a crash");
    }

    #[test]
    fn the_rewrites_leave_other_text_alone() {
        assert_eq!(shim(r"x^2+y"), r"x^2+y");
        assert!(shim(r"a \bmod n").contains(r"\;\mathrm{mod}\;"));
        assert!(shim(r"x \pmod{n+1}").contains(r"\mathrm{mod}\ n+1"));
        assert!(shim(r"\substack{a\\b}").starts_with(r"\begin{array}{c}a\\b"));
        assert_eq!(shim(r"\ce{H2O}"), r"\mathrm{H_{2}O}");
        let rx = shim(r"\ce{CO2 + 2H2 -> CH4}");
        assert!(!rx.contains("\\ce") && rx.contains("\\rightarrow") && rx.contains("_{2}") && rx.contains("_{4}"), "{rx}");
    }

    #[test]
    fn vec_uses_the_combining_arrow_for_single_letters() {
        assert_eq!(shim(r"\vec{B}"), "B\u{20D7}");
        assert_eq!(shim(r"\vec{\omega}+\vec{J}"), "\\omega\u{20D7}+J\u{20D7}");
        assert_eq!(shim(r"\vec F\cdot d\vec r"), "F\u{20D7}\\cdot d r\u{20D7}");
        assert_eq!(shim(r"\times\vec F+\times\vec{G}"), "\\times F\u{20D7}+\\times G\u{20D7}");
        assert_eq!(shim(r"\vec{AB}"), r"\vec{AB}");
        assert_eq!(shim(r"\vec{\mathbf{F}}"), r"\vec{\mathbf{F}}");
        for l in [r"\vec{B}", r"\vec{\omega}", r"\vec{AB}", r"\vec{\mathbf{F}}"] {
            let (plain, arrow) = (render("B", false).unwrap(), render(l, false).unwrap());
            if l == r"\vec{B}" { assert!(arrow.width_pt < plain.width_pt + 2.0, "arrow wider than the letter: {} vs {}", arrow.width_pt, plain.width_pt); }
        }
    }

    #[test]
    fn chemistry_arrows() {
        for (src, cmd) in [("A <=> B", "rightleftharpoons"), ("A <-> B", "leftrightarrow"), ("A <- B", "leftarrow"), ("A -> B", "rightarrow")] {
            let out = shim(&format!(r"\ce{{{src}}}"));
            assert!(out.contains(&format!("\\{cmd}")) && !out.contains('<') && !out.contains('>'), "{src}: {out}");
        }
    }
}
