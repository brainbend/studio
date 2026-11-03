//! A printable handout: one page per slide, drawn statically (no player, no scripts except the
//! print trigger). Question slides list the question and its answers instead.
use std::fmt::Write;
use brainbend_doc::*;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn css(c: Rgba) -> String {
    format!("rgba({},{},{},{:.3})", c.0, c.1, c.2, c.3 as f32 / 255.0)
}

/// CSS `background` value for a slide background (images are inlined).
fn bg_css(b: Option<&brainbend_doc::Background>, p: &Project) -> String {
    use brainbend_doc::Background::*;
    match b {
        None => "#fff".into(),
        Some(Solid(c)) => css(*c),
        Some(Gradient { angle, stops }) => {
            let list: Vec<String> = stops.iter().map(|(at, c)| format!("{} {:.1}%", css(*c), at * 100.0)).collect();
            format!("linear-gradient({angle}deg,{})", list.join(","))
        }
        Some(Image { asset }) => match p.assets.get(asset) {
            Some(a) => format!("url(data:{};base64,{}) center/cover no-repeat", a.mime, base64(&a.bytes)),
            None => "#fff".into(),
        },
    }
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn text_html(o: &Object, link: Rgba) -> String {
    let mut s = String::new();
    let run = |r: &Run| {
        let mut css_s = String::new();
        // A link is the theme's hyperlink colour, underlined, unless the run sets its own colour or underline.
        let linked = r.link.is_some();
        if let Some(c) = r.color.or(linked.then_some(link)) { let _ = write!(css_s, "color:{};", css(c)); }
        if let Some(z) = r.size { let _ = write!(css_s, "font-size:{z}px;"); }
        if let Some(b) = r.bold { let _ = write!(css_s, "font-weight:{};", if b { 700 } else { 400 }); }
        if let Some(i) = r.italic { let _ = write!(css_s, "font-style:{};", if i { "italic" } else { "normal" }); }
        let deco = match (r.underline.or(linked.then_some(true)), r.strike.unwrap_or(false)) { (Some(true), true) => Some("underline line-through"), (_, true) => Some("line-through"), (Some(true), false) => Some("underline"), (Some(false), false) => Some("none"), _ => None };
        if let Some(d) = deco { let _ = write!(css_s, "text-decoration:{d};"); }
        if let Some(h) = r.highlight { let _ = write!(css_s, "background-color:{};", css(h)); }
        if let Some(z) = r.spacing { let _ = write!(css_s, "letter-spacing:{z}px;"); }
        if let Some(f) = &r.font_family { let _ = write!(css_s, "font-family:'{}',sans-serif;", esc(f)); }
        format!("<span style=\"{}\">{}</span>", css_s, esc(&r.text))
    };
    match &o.rich {
        Some(paras) => {
            for p in paras {
                // A list item hangs: the marker sits at the indent and wrapped lines line up with the text after it.
                let text: String = p.runs.iter().map(|r| r.text.as_str()).collect();
                let m = brainbend_doc::richtext::marker_len(&text);
                let hang = if m > 0 && text.len() > m { if text.trim_start_matches('\u{a0}').starts_with('•') { 1.3 } else { 1.6 } } else { 0.0 };
                let pad = p.indent as f32 * brainbend_doc::INDENT_STEP;
                if hang > 0.0 { let _ = write!(s, "<div style=\"padding-left:calc({pad}px + {hang}em);text-indent:-{hang}em\">"); }
                else if pad > 0.0 { let _ = write!(s, "<div style=\"padding-left:{pad}px\">"); }
                else { s.push_str("<div>"); }
                for (i, r) in p.runs.iter().enumerate() {
                    if hang > 0.0 && i == 0 && r.text.len() == m {
                        let _ = write!(s, "<span style=\"display:inline-block;min-width:{hang}em;text-indent:0\">{}</span>", run(r));
                    } else { s.push_str(&run(r)); }
                }
                // An empty paragraph, or one ending in a line break, still takes the line it asks for.
                let text: String = p.runs.iter().map(|r| r.text.as_str()).collect();
                if text.is_empty() || text.ends_with('\n') { s.push_str("&#8203;"); }
                s.push_str("</div>");
            }
        }
        None => {
            for line in o.text.split('\n') {
                let m = brainbend_doc::richtext::marker_len(line);
                if m > 0 && line.len() > m {
                    let hang = if line.trim_start_matches('\u{a0}').starts_with('•') { 1.3 } else { 1.6 };
                    let _ = write!(s, "<div style=\"padding-left:{hang}em;text-indent:-{hang}em\"><span style=\"display:inline-block;min-width:{hang}em;text-indent:0\">{}</span>{}</div>", esc(&line[..m]), esc(&line[m..]));
                } else {
                    let _ = write!(s, "<div>{}</div>", if line.is_empty() { "&nbsp;".into() } else { esc(line) });
                }
            }
        }
    }
    s
}

/// Width over height of a PNG or JPEG, read from its header.
fn image_aspect(b: &[u8]) -> Option<f32> {
    if b.len() > 24 && b.starts_with(&[137, 80, 78, 71]) {
        let (w, h) = (u32::from_be_bytes(b[16..20].try_into().ok()?), u32::from_be_bytes(b[20..24].try_into().ok()?));
        return (h > 0).then(|| w as f32 / h as f32);
    }
    if b.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 9 < b.len() {
            if b[i] != 0xFF { i += 1; continue; }
            let m = b[i + 1];
            if (0xC0..=0xCF).contains(&m) && !matches!(m, 0xC4 | 0xC8 | 0xCC) {
                let (h, w) = (u16::from_be_bytes([b[i + 5], b[i + 6]]) as f32, u16::from_be_bytes([b[i + 7], b[i + 8]]) as f32);
                return (h > 0.0).then(|| w / h);
            }
            i += 2 + u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        }
    }
    None
}

fn object_html(o: &Object, p: &Project) -> String {
    if o.hidden || matches!(o.kind, Kind::Hotspot | Kind::Unsupported(_)) { return String::new(); }
    let mut style = format!("left:{}px;top:{}px;width:{}px;height:{}px;", o.x, o.y, o.w, o.h);
    // A variable's starting value, for the controls that show it.
    let start = |var: &str, fallback: f32| p.variables.iter().find(|v| v.id == var).and_then(|v| if let Value::Num(n) = v.initial { Some(n as f32) } else { None }).unwrap_or(fallback);
    // A dial is drawn turned to where its variable starts.
    let turn = o.rotation + o.dial.as_ref().map(|d| d.angle(start(&d.variable, d.min))).unwrap_or(0.0);
    if turn != 0.0 { let _ = write!(style, "rotate:{}deg;", turn); }
    let mut inner = String::new();
    if let Kind::Slider(sl) = &o.kind {
        // The track, and the thumb where its variable starts.
        let th = (o.h * sl.thumb_height).max(4.0);
        let pic = sl.thumb.as_ref().and_then(|k| p.assets.get(k));
        let tw = pic.and_then(|a| image_aspect(&a.bytes)).map(|r| th * r).unwrap_or(th);
        let left = sl.fraction(start(&sl.variable, sl.min)) * (o.w - tw).max(0.0);
        if let Some(t) = sl.track { let _ = write!(inner, "<div style=\"position:absolute;left:0;right:0;top:50%;height:4px;margin-top:-2px;border-radius:2px;background:{}\"></div>", css(t)); }
        match pic {
            Some(a) => { let _ = write!(inner, "<img src=\"data:{};base64,{}\" style=\"position:absolute;left:{left}px;top:{}px;height:{th}px\" alt=\"\">", esc(&a.mime), base64(&a.bytes), (o.h - th) / 2.0); }
            None => { let _ = write!(inner, "<div style=\"position:absolute;left:{left}px;top:{}px;width:{th}px;height:{th}px;border-radius:50%;background:{}\"></div>", (o.h - th) / 2.0, css(sl.thumb_color)); }
        }
    }
    if let Some(ck) = &o.check {
        // The box, ticked when the check box starts selected.
        let (bx, by, side) = ck.box_in(o.h);
        let line = ck.box_line.map(|l| format!("1px solid {}", css(l))).unwrap_or_else(|| "none".into());
        let tick = if o.initial_state == "Selected" { format!("<svg viewBox=\"0 0 20 20\" style=\"position:absolute;inset:0;width:100%;height:100%\"><path d=\"M4.5 10.5l3.5 3.5 7.5-8\" fill=\"none\" stroke=\"{}\" stroke-width=\"2.2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg>", css(ck.tick)) } else { String::new() };
        let _ = write!(inner, "<div style=\"position:absolute;box-sizing:border-box;border-radius:3px;left:{bx}px;top:{by}px;width:{side}px;height:{side}px;background:{};border:{line}\">{tick}</div>", css(ck.box_fill));
    }
    match &o.kind {
        Kind::Ellipse => style.push_str("border-radius:50%;"),
        _ if o.radius > 0.0 => { let _ = write!(style, "border-radius:{}px;", o.radius); }
        _ => {}
    }
    match &o.kind {
        Kind::Path(cmds) => {
            let d: String = cmds.iter().map(|c| match c {
                PathCmd::Move(x, y) => format!("M{x} {y}"),
                PathCmd::Line(x, y) => format!("L{x} {y}"),
                PathCmd::Curve(a, b, c2, d2, e, f) => format!("C{a} {b} {c2} {d2} {e} {f}"),
                PathCmd::Close => "Z".into(),
            }).collect();
            let fill = o.fill.map(css).unwrap_or_else(|| "none".into());
            let (stroke, sw) = o.stroke.map(|s| (css(s.color), s.width)).unwrap_or(("none".into(), 0.0));
            let _ = write!(inner, "<svg viewBox=\"0 0 1 1\" preserveAspectRatio=\"none\" width=\"100%\" height=\"100%\" style=\"overflow:visible\"><path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\" vector-effect=\"non-scaling-stroke\"/></svg>");
        }
        Kind::Line | Kind::LineUp => {
            let (y1, y2) = if matches!(o.kind, Kind::Line) { (0.0, o.h) } else { (o.h, 0.0) };
            let (stroke, sw) = o.stroke.map(|s| (css(s.color), s.width)).unwrap_or(("none".into(), 0.0));
            let _ = write!(inner, "<svg width=\"100%\" height=\"100%\" style=\"overflow:visible\"><line x1=\"0\" y1=\"{y1}\" x2=\"{}\" y2=\"{y2}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/></svg>", o.w);
        }
        _ => {
            if let Some(g) = &o.gradient {
                let stops: Vec<String> = g.stops.iter().map(|(at, c)| format!("{} {:.1}%", css(*c), at * 100.0)).collect();
                let _ = write!(style, "background:linear-gradient({}deg,{});", g.angle, stops.join(","));
            } else if let Some(f) = o.fill { let _ = write!(style, "background:{};", css(f)); }
            if let Some(b) = o.table_borders {
                for (name, side) in ["left", "right", "top", "bottom"].iter().zip(b.iter()) {
                    if let Some(s) = side { let _ = write!(style, "border-{name}:{}px solid {};", s.width, css(s.color)); }
                }
                style.push_str("box-sizing:border-box;");
            } else if let Some(s) = o.stroke { let _ = write!(style, "border:{}px {} {};box-sizing:border-box;", s.width, o.dash.css(), css(s.color)); }
            let mut shadows: Vec<String> = Vec::new();
            if let Some(sh) = o.shadow { shadows.push(format!("{}px {}px {}px {}px {}", sh.dx, sh.dy, sh.blur, sh.spread, css(sh.color))); }
            if let Some(g) = o.glow { shadows.push(format!("0 0 {}px {}px {}", g.radius, (g.radius / 4.0).round(), css(g.color))); }
            if !shadows.is_empty() { let _ = write!(style, "box-shadow:{};", shadows.join(",")); }
        }
    }
    if matches!(o.kind, Kind::Image) {
        if let Some(a) = o.asset.as_ref().and_then(|k| p.assets.get(k)) {
            // A picture cropped to a shape: the wrapper shows only what is inside it.
            if let Some(shape) = o.mask { let _ = write!(inner, "<div style=\"width:100%;height:100%;clip-path:{}\">", shape.css_polygon(o.w, o.h)); }
            match o.crop {
                Some(c) => {
                    let [x, y, w, h] = brainbend_doc::crop_layout(c);
                    let _ = write!(inner, "<div style=\"position:relative;width:100%;height:100%;overflow:hidden\"><img src=\"data:{};base64,{}\" style=\"position:absolute;max-width:none;left:{}%;top:{}%;width:{}%;height:{}%\" alt=\"{}\"></div>", esc(&a.mime), base64(&a.bytes), x * 100.0, y * 100.0, w * 100.0, h * 100.0, esc(&o.alt));
                }
                None => { let _ = write!(inner, "<img src=\"data:{};base64,{}\" style=\"width:100%;height:100%;display:block\" alt=\"{}\">", esc(&a.mime), base64(&a.bytes), esc(&o.alt)); }
            }
            if o.mask.is_some() { inner.push_str("</div>"); }
        }
    }
    if let Kind::Input(f) = &o.kind {
        let _ = write!(inner, "<div class=\"media\">{} field: {}</div>", if f.numeric { "Number" } else { "Text" }, esc(&f.placeholder));
    }
    if let Kind::Web(w) = &o.kind {
        let _ = write!(inner, "<div class=\"media\">Web page: {}</div>", esc(w.url.trim()));
    }
    if let Kind::Pano(p) = &o.kind {
        let _ = write!(inner, "<div class=\"media\">360° image: {}. Look around it in the published lesson.", esc(if o.alt.trim().is_empty() { &o.name } else { o.alt.trim() }));
        for pin in &p.pins { let _ = write!(inner, "<br>{}{}{}", esc(&pin.title), if pin.description.is_empty() { "" } else { ": " }, esc(&pin.description)); }
        inner.push_str("</div>");
    }
    if matches!(o.kind, Kind::Video | Kind::Audio) {
        let _ = write!(inner, "<div class=\"media\">{} {}</div>", if matches!(o.kind, Kind::Video) { "Video:" } else { "Audio:" }, esc(&o.name));
    }
    if !o.text.is_empty() && !matches!(o.kind, Kind::Path(_) | Kind::Line | Kind::LineUp) {
        let st = &o.style;
        let justify = match st.valign { VAlign::Top => "flex-start", VAlign::Middle => "center", VAlign::Bottom => "flex-end" };
        let tag = match o.role { TextRole::H1 => "h1", TextRole::H2 => "h2", TextRole::H3 => "h3", TextRole::H4 => "h4", TextRole::Normal => "div" };
        let _ = write!(
            inner,
            "<{tag} class=\"label\" style=\"margin:0;justify-content:{justify};text-align:{};padding:{}px {}px {}px {}px;color:{};font:{} {}px/{} '{}',sans-serif;{}{}\">{}</{tag}>",
            match st.align { Align::Left => "left", Align::Center => "center", Align::Right => "right", Align::Justify => "justify" },
            o.margins[1], o.margins[2], o.margins[3], o.margins[0],
            css(st.color),
            if st.italic { "italic" } else { "normal" },
            st.size,
            if o.line_spacing > 0.0 { o.line_spacing } else { 1.2 },
            esc(&st.font_family),
            if st.bold { "font-weight:700;" } else { "" },
            if st.underline { "text-decoration:underline;" } else { "" },
            text_html(o, p.theme.color(4)),
        );
    }
    format!("<div class=\"obj\" style=\"{style}\">{inner}</div>")
}

fn question_html(q: &Question, project: &Project) -> String {
    let mut s = format!("<div class=\"qa\"><h2>{}</h2>", esc(&q.prompt));
    match &q.style {
        QuestionStyle::Choice => {
            s.push_str("<ul>");
            for c in &q.choices {
                let _ = write!(s, "<li>{} {}</li>", if c.correct { "&#9745;" } else { "&#9744;" }, esc(&c.text));
            }
            s.push_str("</ul><p class=\"note\">&#9745; marks a correct answer.</p>");
        }
        QuestionStyle::TextEntry { accepted, .. } => {
            let _ = write!(s, "<p>Accepted answers: {}</p>", accepted.iter().map(|a| esc(a)).collect::<Vec<_>>().join(", "));
        }
        QuestionStyle::Matching { pairs } => {
            s.push_str("<table>");
            for (a, b) in pairs { let _ = write!(s, "<tr><td>{}</td><td>&rarr;</td><td>{}</td></tr>", esc(a), esc(b)); }
            s.push_str("</table>");
        }
        QuestionStyle::Sequence { items } => {
            s.push_str("<ol>");
            for i in items { let _ = write!(s, "<li>{}</li>", esc(i)); }
            s.push_str("</ol>");
        }
        QuestionStyle::FillBlanks { text, .. } => {
            let _ = write!(s, "<p>{}</p><p class=\"note\">Answers: {}</p>", esc(&brainbend_doc::passage::with_gaps(text)),
                brainbend_doc::passage::blanks(text).iter().map(|a| esc(&a.join(" / "))).collect::<Vec<_>>().join(", "));
        }
        QuestionStyle::DragWords { text, distractors } => {
            let mut bank: Vec<String> = brainbend_doc::passage::blanks(text).into_iter().map(|a| a[0].clone()).collect();
            bank.extend(distractors.iter().cloned());
            let _ = write!(s, "<p>{}</p><p class=\"note\">Word bank: {}</p>", esc(&brainbend_doc::passage::with_gaps(text)), bank.iter().map(|w| esc(w)).collect::<Vec<_>>().join(", "));
        }
        QuestionStyle::MarkWords { text } => {
            s.push_str("<p>");
            for part in brainbend_doc::passage::parse(text) {
                match part {
                    brainbend_doc::passage::Part::Text(t) => s.push_str(&esc(&t)),
                    brainbend_doc::passage::Part::Blank(a) => { let _ = write!(s, "<strong>{}</strong>", esc(&a[0])); }
                }
            }
            s.push_str("</p><p class=\"note\">Bold words are the ones to mark.</p>");
        }
        QuestionStyle::Numeric { answer, tolerance } => {
            let _ = write!(s, "<p>Answer: {answer}{}</p>", if *tolerance > 0.0 { format!(" (within {tolerance})") } else { String::new() });
        }
        QuestionStyle::Hotspot { asset, alt, regions } => {
            if let Some(a) = asset.as_ref().and_then(|k| project.assets.get(k)) {
                let _ = write!(s, "<div style=\"position:relative;display:inline-block\"><img src=\"data:{};base64,{}\" style=\"max-width:100%;max-height:420px;display:block\" alt=\"{}\">", esc(&a.mime), base64(&a.bytes), esc(alt));
                for r in regions {
                    let _ = write!(s, "<div style=\"position:absolute;left:{:.2}%;top:{:.2}%;width:{:.2}%;height:{:.2}%;border:3px solid #1e8e3e;box-sizing:border-box;{}\"></div>", r.x * 100.0, r.y * 100.0, r.w * 100.0, r.h * 100.0, if r.ellipse { "border-radius:50%" } else { "" });
                }
                s.push_str("</div><p class=\"note\">The green outline marks the correct area.</p>");
            }
        }
        QuestionStyle::Likert { statements, scale, .. } => {
            let _ = write!(s, "<table><tr><th></th>{}</tr>", scale.iter().map(|c| format!("<th>{}</th>", esc(c))).collect::<String>());
            for st in statements { let _ = write!(s, "<tr><td>{}</td>{}</tr>", esc(st), "<td>&#9744;</td>".repeat(scale.len())); }
            s.push_str("</table><p class=\"note\">Survey: not graded.</p>");
        }
        QuestionStyle::Reflection { min_words } => {
            let _ = write!(s, "<p style=\"border:1px solid #999;min-height:120px\"></p><p class=\"note\">Free-text answer, not graded.{}</p>", if *min_words > 0 { format!(" At least {min_words} words.") } else { String::new() });
        }
        QuestionStyle::Dictation { text, .. } => {
            let _ = write!(s, "<p>Audio question. Transcript: {}</p>", esc(text));
        }
        QuestionStyle::Dropdowns { text } => {
            let mut t = String::new();
            for part in brainbend_doc::passage::parse(text) {
                match part {
                    brainbend_doc::passage::Part::Text(x) => t.push_str(&esc(&x)),
                    brainbend_doc::passage::Part::Blank(a) => { let _ = write!(t, "[<strong>{}</strong>{}]", esc(&a[0]), a[1..].iter().map(|w| format!(" / {}", esc(w))).collect::<String>()); }
                }
            }
            let _ = write!(s, "<p>{t}</p><p class=\"note\">The bold option is correct.</p>");
        }
        QuestionStyle::LabelPicture { asset, alt, zones, distractors } => {
            if let Some(a) = asset.as_ref().and_then(|k| project.assets.get(k)) {
                let _ = write!(s, "<div style=\"position:relative;display:inline-block\"><img src=\"data:{};base64,{}\" style=\"max-width:100%;max-height:420px;display:block\" alt=\"{}\">", esc(&a.mime), base64(&a.bytes), esc(alt));
                for (i, z) in zones.iter().enumerate() {
                    let r = z.region;
                    let _ = write!(s, "<div style=\"position:absolute;left:{:.2}%;top:{:.2}%;width:{:.2}%;height:{:.2}%;border:3px solid #1e8e3e;box-sizing:border-box;font:700 16px sans-serif;color:#1e8e3e\">{}</div>", r.x * 100.0, r.y * 100.0, r.w * 100.0, r.h * 100.0, i + 1);
                }
                s.push_str("</div>");
            }
            let _ = write!(s, "<ol>{}</ol>", zones.iter().map(|z| format!("<li>{}</li>", esc(&z.label))).collect::<String>());
            if !distractors.is_empty() { let _ = write!(s, "<p class=\"note\">Spare labels: {}</p>", distractors.iter().map(|d| esc(d)).collect::<Vec<_>>().join(", ")); }
        }
        QuestionStyle::Categorize { categories } => {
            s.push_str("<table>");
            for (name, items) in categories { let _ = write!(s, "<tr><td><strong>{}</strong></td><td>{}</td></tr>", esc(name), items.iter().map(|i| esc(i)).collect::<Vec<_>>().join(", ")); }
            s.push_str("</table>");
        }
    }
    let _ = write!(s, "<p class=\"note\">Worth {} point(s). Correct: {} Incorrect: {}</p></div>", q.points, esc(&q.feedback_correct), esc(&q.feedback_incorrect));
    s
}

/// Build the printable document; opening it in a browser shows the print dialog.
pub fn print_html(project: &Project) -> String {
    let flat = project.flattened();
    let p = &flat;
    let mut pages = String::new();
    for (i, slide) in p.slides.iter().enumerate() {
        // Page content is shrunk to a 960px wide sheet (about a landscape letter/A4 page).
        let k = 960.0 / slide.width;
        let bg = bg_css(slide.background.as_ref(), p);
        let body = match &slide.kind {
            SlideKind::Normal => slide.layers.first().map(|l| l.objects.iter().map(|o| object_html(o, p)).collect::<String>()).unwrap_or_default(),
            SlideKind::Question(q) => question_html(q, p),
            SlideKind::Results => "<div class=\"qa\"><h2>Results</h2><p>The player shows the learner's score, pass or fail, and a Retry button here.</p></div>".to_string(),
        };
        let _ = write!(
            pages,
            "<section class=\"page\"><div class=\"cap\">{}. {}</div><div class=\"frame\" style=\"width:960px;height:{}px\"><div class=\"slide\" style=\"width:{}px;height:{}px;background:{};transform:scale({k})\">{}</div></div></section>",
            i + 1, esc(&slide.name), slide.height * k, slide.width, slide.height, bg, body
        );
    }
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title><style>\
@page{{size:landscape;margin:12mm}}\
body{{font-family:'Open Sans',Arial,sans-serif;margin:0;color:#222}}\
.page{{page-break-after:always;padding:10px 0;break-inside:avoid}}\
.cap{{font-size:12px;color:#666;margin:0 0 6px 2px}}\
.frame{{position:relative;overflow:hidden;border:1px solid #bbb;margin:0 auto}}\
.slide{{position:absolute;left:0;top:0;transform-origin:0 0;overflow:hidden}}\
.obj{{position:absolute}}\
.label{{position:absolute;inset:0;display:flex;flex-direction:column;box-sizing:border-box;overflow:hidden}}\
.media{{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;background:#333;color:#ddd;font-size:22px}}\
.qa{{padding:40px 60px;font-size:18px}}.qa h2{{margin-top:0}}.qa li{{margin:6px 0;list-style:none}}.qa ol li{{list-style:decimal}}.note{{color:#666;font-size:14px}}\
@media print{{.page{{padding:0}}}}\
</style></head><body>{pages}<script>window.addEventListener('load',function(){{setTimeout(function(){{window.print();}},500);}});</script></body></html>",
        title = esc(&p.name),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }

    #[test]
    fn linked_text_prints_in_the_link_colour_underlined() {
        let mut o = Object::new("t", "Text", Kind::Text, 0.0, 0.0, 100.0, 20.0);
        o.text = "go now".into();
        o.rich = Some(vec![Paragraph { indent: 0, runs: vec![
            Run { text: "go".into(), link: Some("https://example.com".into()), ..Default::default() },
            Run { text: " now".into(), ..Default::default() },
        ] }]);
        let html = text_html(&o, Rgba(1, 2, 3, 255));
        assert!(html.contains("color:rgba(1,2,3,1);text-decoration:underline;\">go</span>") || html.contains("text-decoration:underline;\">go</span>"), "{html}");
        assert!(html.contains("<span style=\"\"> now</span>"), "{html}");
    }

    #[test]
    fn one_page_per_slide_with_escaped_names() {
        let mut p = Project::new("A <b> & C");
        p.slides.push(Slide::new("s2", "Second", 1920.0, 1080.0));
        let html = print_html(&p);
        assert_eq!(html.matches("class=\"page\"").count(), p.slides.len());
        assert!(html.contains("A &lt;b&gt; &amp; C"));
        assert!(html.contains("window.print()"));
    }

    #[test]
    fn controls_print_as_they_start() {
        let mut p = Project::new("t");
        p.slides.push(Slide::new("s1", "One", 1920.0, 1080.0));
        p.variables.push(Variable { id: "v".into(), name: "Level".into(), initial: Value::Num(5.0), built_in: false });
        let slider = Object::new("s", "Slider", Kind::Slider(Slider { variable: "v".into(), min: 0.0, max: 10.0, thumb_height: 0.5, ..Slider::new() }), 0.0, 0.0, 100.0, 20.0);
        let mut dial = Object::new("d", "Dial", Kind::Rect, 0.0, 50.0, 40.0, 40.0);
        dial.dial = Some(Dial { variable: "v".into(), min: 0.0, max: 10.0, step: 1.0, sweep: 180.0, pivot: (0.0, 0.0) });
        let mut tilted = dial.clone();
        tilted.id = "t".into();
        tilted.rotation = 10.0;
        tilted.dial.as_mut().unwrap().max = 20.0; // 5 of 0..20 is a quarter of the way: -45 degrees
        p.slides[0].layers[0].objects = vec![slider, dial, tilted];
        let html = print_html(&p);
        // A 10 tall thumb on a 100 wide track, half way along: (100 - 10) / 2 from the left.
        assert!(html.contains("left:45px;top:5px;width:10px;height:10px;border-radius:50%"), "{html}");
        assert_eq!(html.matches("rotate:").count(), 1, "the middle of a dial's range is upright, so only the other dial is turned");
        assert!(html.contains("rotate:-35deg"), "a turned dial adds its turn to its own rotation: 10 - 45");
    }

    #[test]
    fn a_picture_cropped_to_a_shape_prints_clipped_to_it() {
        let mut p = Project::new("t");
        p.slides.push(Slide::new("s1", "One", 1920.0, 1080.0));
        p.assets.insert("pic".into(), Asset { name: "p.png".into(), mime: "image/png".into(), bytes: vec![137, 80, 78, 71], file: None });
        let mut o = Object::new("i", "Pic", Kind::Image, 0.0, 0.0, 200.0, 200.0);
        o.asset = Some("pic".into());
        o.mask = Some(PictureShape::Diamond);
        p.slides[0].layers[0].objects = vec![o];
        let html = print_html(&p);
        assert!(html.contains("clip-path:polygon(50.00% 0.00%,100.00% 50.00%,50.00% 100.00%,0.00% 50.00%)"), "{html}");
        p.slides[0].layers[0].objects[0].mask = None;
        assert!(!print_html(&p).contains("clip-path:polygon"));
    }
}
