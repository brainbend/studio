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
fn bg_css(b: Option<&brainbend_doc::Background>, p: &Project, w: f32, h: f32) -> String {
    use brainbend_doc::Background::*;
    match b {
        None => "#fff".into(),
        Some(Solid(c)) => css(*c),
        Some(Gradient { angle, stops, kind, geom }) => brainbend_doc::gradient_css(*kind, *angle, stops, geom, w, h),
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
        // A link is the theme's hyperlink color, underlined, unless the run sets its own color or underline.
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

/// The question as a handout shows it. With `answers` the correct answers are shown (ticked, outlined, listed); without,
/// it is a blank worksheet, and anything that would give an answer away by its order is put in alphabetical order.
fn question_html(q: &Question, project: &Project, answers: bool) -> String {
    let sorted = |items: Vec<String>| { let mut v = items; v.sort_by_key(|x| x.to_lowercase()); v };
    let mut s = format!("<div class=\"qa\"><h2>{}</h2>", esc(&q.prompt));
    match &q.style {
        QuestionStyle::Choice => {
            s.push_str("<ul>");
            for c in &q.choices {
                let _ = write!(s, "<li>{} {}</li>", if answers && c.correct { "&#9745;" } else { "&#9744;" }, esc(&c.text));
            }
            s.push_str("</ul>");
            if answers { s.push_str("<p class=\"note\">&#9745; marks a correct answer.</p>"); }
        }
        QuestionStyle::TextEntry { accepted, .. } => {
            if answers { let _ = write!(s, "<p>Accepted answers: {}</p>", accepted.iter().map(|a| esc(a)).collect::<Vec<_>>().join(", ")); }
            else { s.push_str("<p>Answer: ______________________</p>"); }
        }
        QuestionStyle::Matching { pairs } => {
            s.push_str("<table>");
            for (a, b) in pairs { let _ = write!(s, "<tr><td>{}</td><td>&rarr;</td><td>{}</td></tr>", esc(a), if answers { esc(b) } else { "______________".to_string() }); }
            s.push_str("</table>");
            if !answers { let _ = write!(s, "<p class=\"note\">Match each one with: {}</p>", sorted(pairs.iter().map(|(_, b)| esc(b)).collect()).join(", ")); }
        }
        QuestionStyle::Sequence { items } => {
            if answers {
                s.push_str("<ol>");
                for i in items { let _ = write!(s, "<li>{}</li>", esc(i)); }
                s.push_str("</ol>");
            } else {
                let _ = write!(s, "<p>Put in order: {}</p>", sorted(items.iter().map(|i| esc(i)).collect()).join(", "));
            }
        }
        QuestionStyle::FillBlanks { text, .. } => {
            let _ = write!(s, "<p>{}</p>", esc(&brainbend_doc::passage::with_gaps(text)));
            if answers { let _ = write!(s, "<p class=\"note\">Answers: {}</p>", brainbend_doc::passage::blanks(text).iter().map(|a| esc(&a.join(" / "))).collect::<Vec<_>>().join(", ")); }
        }
        QuestionStyle::DragWords { text, distractors } => {
            let mut bank: Vec<String> = brainbend_doc::passage::blanks(text).into_iter().map(|a| a[0].clone()).collect();
            bank.extend(distractors.iter().cloned());
            if !answers { bank = sorted(bank); }
            let _ = write!(s, "<p>{}</p><p class=\"note\">Word bank: {}</p>", esc(&brainbend_doc::passage::with_gaps(text)), bank.iter().map(|w| esc(w)).collect::<Vec<_>>().join(", "));
        }
        QuestionStyle::MarkWords { text } => {
            s.push_str("<p>");
            for part in brainbend_doc::passage::parse(text) {
                match part {
                    brainbend_doc::passage::Part::Text(t) => s.push_str(&esc(&t)),
                    brainbend_doc::passage::Part::Blank(a) => { if answers { let _ = write!(s, "<strong>{}</strong>", esc(&a[0])); } else { s.push_str(&esc(&a[0])); } }
                }
            }
            s.push_str(if answers { "</p><p class=\"note\">Bold words are the ones to mark.</p>" } else { "</p><p class=\"note\">Mark the correct words.</p>" });
        }
        QuestionStyle::Numeric { answer, tolerance } => {
            if answers { let _ = write!(s, "<p>Answer: {answer}{}</p>", if *tolerance > 0.0 { format!(" (within {tolerance})") } else { String::new() }); }
            else { s.push_str("<p>Answer: ______________</p>"); }
        }
        QuestionStyle::Hotspot { asset, alt, regions } => {
            if let Some(a) = asset.as_ref().and_then(|k| project.assets.get(k)) {
                let _ = write!(s, "<div style=\"position:relative;display:inline-block\"><img src=\"data:{};base64,{}\" style=\"max-width:100%;max-height:420px;display:block\" alt=\"{}\">", esc(&a.mime), base64(&a.bytes), esc(alt));
                if answers {
                    for r in regions {
                        let _ = write!(s, "<div style=\"position:absolute;left:{:.2}%;top:{:.2}%;width:{:.2}%;height:{:.2}%;border:3px solid #1e8e3e;box-sizing:border-box;{}\"></div>", r.x * 100.0, r.y * 100.0, r.w * 100.0, r.h * 100.0, if r.ellipse { "border-radius:50%" } else { "" });
                    }
                }
                s.push_str(if answers { "</div><p class=\"note\">The green outline marks the correct area.</p>" } else { "</div><p class=\"note\">Mark the correct area.</p>" });
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
            if answers { let _ = write!(s, "<p>Audio question. Transcript: {}</p>", esc(text)); } else { s.push_str("<p>Audio question. Write what you hear: ______________________</p>"); }
        }
        QuestionStyle::Dropdowns { text } => {
            let mut t = String::new();
            for part in brainbend_doc::passage::parse(text) {
                match part {
                    brainbend_doc::passage::Part::Text(x) => t.push_str(&esc(&x)),
                    brainbend_doc::passage::Part::Blank(a) => {
                        if answers { let _ = write!(t, "[<strong>{}</strong>{}]", esc(&a[0]), a[1..].iter().map(|w| format!(" / {}", esc(w))).collect::<String>()); }
                        else { let _ = write!(t, "[{}]", sorted(a.iter().map(|w| esc(w)).collect()).join(" / ")); }
                    }
                }
            }
            let _ = write!(s, "<p>{t}</p>");
            if answers { s.push_str("<p class=\"note\">The bold option is correct.</p>"); }
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
            if answers {
                let _ = write!(s, "<ol>{}</ol>", zones.iter().map(|z| format!("<li>{}</li>", esc(&z.label))).collect::<String>());
                if !distractors.is_empty() { let _ = write!(s, "<p class=\"note\">Spare labels: {}</p>", distractors.iter().map(|d| esc(d)).collect::<Vec<_>>().join(", ")); }
            } else {
                let labels: Vec<String> = zones.iter().map(|z| z.label.clone()).chain(distractors.iter().cloned()).collect();
                let _ = write!(s, "<p class=\"note\">Label the numbered areas with: {}</p>", sorted(labels.iter().map(|l| esc(l)).collect()).join(", "));
            }
        }
        QuestionStyle::Categorize { categories } => {
            s.push_str("<table>");
            for (name, items) in categories { let _ = write!(s, "<tr><td><strong>{}</strong></td><td>{}</td></tr>", esc(name), if answers { items.iter().map(|i| esc(i)).collect::<Vec<_>>().join(", ") } else { "&nbsp;".to_string() }); }
            s.push_str("</table>");
            if !answers { let _ = write!(s, "<p class=\"note\">Sort these: {}</p>", sorted(categories.iter().flat_map(|(_, items)| items.iter().map(|i| esc(i))).collect()).join(", ")); }
        }
    }
    let _ = write!(s, "<p class=\"note\">Worth {} point(s). Correct: {} Incorrect: {}</p></div>", q.points, esc(&q.feedback_correct), esc(&q.feedback_incorrect));
    s
}

/// Which way up the handout's pages are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HandoutOrientation {
    #[default]
    Landscape,
    Portrait,
}

impl HandoutOrientation {
    pub const ALL: [HandoutOrientation; 2] = [Self::Landscape, Self::Portrait];

    pub fn label(self) -> &'static str {
        match self { Self::Landscape => "Landscape", Self::Portrait => "Portrait" }
    }
}

/// What the handout holds and how it is laid out. The author chooses these; the rest (slide sizes, borders, spacing, where
/// the note lines go) follows from them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandoutOptions {
    pub orientation: HandoutOrientation,
    /// Slides on each page: 1, 2 or 3.
    pub per_page: u8,
    /// Show the correct answers on question slides. Off gives a blank worksheet.
    pub correct_answers: bool,
    /// Lined space to write on, beside or below each slide.
    pub note_lines: bool,
    /// Print each slide's notes with it.
    pub slide_notes: bool,
    /// Print the alt text of a slide's pictures and shapes with it.
    pub alt_text: bool,
}

impl HandoutOptions {
    /// The numbers of slides a page can hold.
    pub const PER_PAGE: [u8; 3] = [1, 2, 3];
}

impl Default for HandoutOptions {
    fn default() -> Self {
        HandoutOptions { orientation: HandoutOrientation::Landscape, per_page: 1, correct_answers: true, note_lines: false, slide_notes: false, alt_text: false }
    }
}

/// CSS pixels in a millimetre.
const PX_PER_MM: f32 = 96.0 / 25.4;
/// Space between slides on a page, and the height of a slide's caption line, in millimetres.
const GAP_MM: f32 = 6.0;
const CAPTION_MM: f32 = 7.0;
/// What a slide's notes area needs under it when slides are stacked: room for some lines or a few notes.
const NOTES_BELOW_MM: f32 = 28.0;

/// Build the printable document; opening it in a browser shows the print dialog.
/// `@font-face` rules for the fonts the handout uses, with the files inside the page, so the handout looks right
/// in any browser (it is a single file opened from a temporary folder).
fn handout_fonts(p: &Project) -> String {
    let mut used = p.fonts_used();
    used.insert("Open Sans".into());
    let e = brainbend_fonts::embed(&used, "");
    let mut css = e.css;
    for (path, bytes) in &e.files {
        let Some(file) = path.strip_prefix("fonts/") else { continue };
        if file.ends_with(".txt") { continue; }
        let mime = if file.ends_with(".otf") { "font/otf" } else { "font/ttf" };
        css = css.replace(&format!("url({file})"), &format!("url(data:{mime};base64,{})", base64(bytes)));
    }
    css.replace('\n', "")
}

pub fn print_html(project: &Project) -> String { print_html_with(project, &HandoutOptions::default()) }

/// The handout laid out as `options` say. Sizes are in millimetres and leave room for the browser's margins, so the pages
/// fit A4 and Letter alike.
pub fn print_html_with(project: &Project, options: &HandoutOptions) -> String {
    let flat = project.flattened();
    let p = &flat;
    let per_page = options.per_page.clamp(1, 3) as usize;
    let portrait = options.orientation == HandoutOrientation::Portrait;
    let has_notes_area = options.note_lines || options.slide_notes || options.alt_text;
    // The page, as a grid of cells: portrait stacks the slides, landscape puts them side by side. A slide that has notes
    // area to itself either sits above it (stacked) or beside it (when the cell is wide and short, or a column of three).
    let (page_w, page_h): (f32, f32) = if portrait { (180.0, 248.0) } else { (265.0, 178.0) };
    let (cols, rows) = if portrait { (1, per_page) } else { (per_page, 1) };
    let beside = if portrait { per_page == 3 } else { per_page == 1 && has_notes_area };
    let cell_w = (page_w - GAP_MM * (cols as f32 - 1.0)) / cols as f32;
    let cell_h = (page_h - GAP_MM * (rows as f32 - 1.0)) / rows as f32;
    let alone = !beside && !has_notes_area && !portrait; // a landscape slide with nothing else: centred
    let mut cells: Vec<String> = vec![];
    for (i, slide) in p.slides.iter().enumerate() {
        let bg = bg_css(slide.background.as_ref(), p, slide.width, slide.height);
        let objects = slide.layers.first().map(|l| l.objects.as_slice()).unwrap_or(&[]);
        let body = match &slide.kind {
            SlideKind::Normal => objects.iter().map(|o| object_html(o, p)).collect::<String>(),
            SlideKind::Question(q) => question_html(q, p, options.correct_answers),
            SlideKind::Results => "<div class=\"qa\"><h2>Results</h2><p>The player shows the learner's score, pass or fail, and a Retry button here.</p></div>".to_string(),
        };
        let aspect = slide.height / slide.width;
        let frame_w = if beside {
            (cell_w * 0.58).min((cell_h - CAPTION_MM) / aspect)
        } else {
            let below = if has_notes_area { NOTES_BELOW_MM } else { 0.0 };
            cell_w.min((cell_h - CAPTION_MM - below) / aspect)
        };
        let frame_h = frame_w * aspect;
        let k = frame_w * PX_PER_MM / slide.width;
        // What goes with the slide: its notes, its alt text, and lines to write on.
        let mut extras = String::new();
        if options.slide_notes && !slide.notes.trim().is_empty() {
            let _ = write!(extras, "<div class=\"x\"><strong>Notes</strong> {}</div>", esc(slide.notes.trim()));
        }
        if options.alt_text {
            let items: String = objects.iter().filter(|o| !o.decorative && !o.alt.trim().is_empty()).map(|o| format!("<li>{}: {}</li>", esc(&o.name), esc(o.alt.trim()))).collect();
            if !items.is_empty() { let _ = write!(extras, "<div class=\"x\"><strong>Alt text</strong><ul>{items}</ul></div>"); }
        }
        let lines = if options.note_lines { "<div class=\"lines\"></div>" } else { "" };
        let slide_html = format!(
            "<div class=\"cap\">{}. {}</div><div class=\"frame\" style=\"width:{frame_w:.2}mm;height:{frame_h:.2}mm\"><div class=\"slide\" style=\"width:{}px;height:{}px;background:{};transform:scale({k})\">{}</div></div>",
            i + 1, esc(&slide.name), slide.width, slide.height, bg, body
        );
        let cell = if beside {
            format!("<div class=\"cell beside\"><div class=\"left\">{slide_html}</div><div class=\"na\" style=\"height:{frame_h:.2}mm\">{extras}{lines}</div></div>")
        } else if has_notes_area {
            format!("<div class=\"cell stack\">{slide_html}<div class=\"na\" style=\"width:{frame_w:.2}mm\">{extras}{lines}</div></div>")
        } else {
            format!("<div class=\"cell stack{}\">{slide_html}</div>", if alone { " center" } else { "" })
        };
        cells.push(cell);
    }
    let pages: String = cells.chunks(per_page).map(|group| format!(
        "<section class=\"page\" style=\"width:{page_w}mm;height:{page_h}mm;grid-template-columns:repeat({cols},1fr);grid-template-rows:repeat({rows},1fr);gap:{GAP_MM}mm\">{}</section>", group.concat()
    )).collect();
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title><style>\
{fonts}\
@page{{size:{orientation};margin:12mm}}\
*{{-webkit-print-color-adjust:exact;print-color-adjust:exact}}\
body{{font-family:'Open Sans',Arial,sans-serif;margin:0;color:#222}}\
.page{{display:grid;page-break-after:always;break-inside:avoid;box-sizing:border-box}}\
@media screen{{.page{{margin:0 auto 10mm}}}}\
.cap{{font-size:12px;color:#666;height:{cap}mm;box-sizing:border-box;padding-left:2px}}\
.frame{{position:relative;overflow:hidden;border:1px solid #888;box-sizing:content-box}}\
.cell{{break-inside:avoid;min-width:0;min-height:0}}\
.cell.stack{{display:flex;flex-direction:column;align-items:flex-start}}\
.cell.stack.center{{align-items:center}}\
.cell.beside{{display:flex;gap:8mm;align-items:flex-start}}\
.cell.beside .na{{flex:1;margin-top:{cap}mm}}\
.na{{display:flex;flex-direction:column;min-height:0;min-width:0;flex:1}}\
.cell.stack .na{{width:100%}}\
.x{{font-size:11px;line-height:1.35;margin:1mm 0 2mm;white-space:pre-wrap;overflow:hidden}}\
.x ul{{margin:2px 0 0;padding-left:16px}}\
.lines{{flex:1;min-height:10mm;background:repeating-linear-gradient(to bottom,transparent 0,transparent 7.2mm,#aaa 7.2mm,#aaa 7.5mm)}}\
.slide{{position:absolute;left:0;top:0;transform-origin:0 0;overflow:hidden}}\
.obj{{position:absolute}}\
.label{{position:absolute;inset:0;display:flex;flex-direction:column;box-sizing:border-box;overflow:hidden}}\
.media{{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;background:#333;color:#ddd;font-size:22px}}\
.qa{{padding:40px 60px;font-size:18px}}.qa h2{{margin-top:0}}.qa li{{margin:6px 0;list-style:none}}.qa ol li{{list-style:decimal}}.note{{color:#666;font-size:14px}}\
</style></head><body>{pages}<script>window.addEventListener('load',function(){{setTimeout(function(){{window.print();}},500);}});</script></body></html>",
        title = esc(&p.name),
        fonts = handout_fonts(p),
        orientation = if portrait { "portrait" } else { "landscape" },
        cap = CAPTION_MM,
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
    fn linked_text_prints_in_the_link_color_underlined() {
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
        assert_eq!(html.matches("<section class=\"page\"").count(), p.slides.len());
        assert!(html.contains("A &lt;b&gt; &amp; C"));
        assert!(html.contains("window.print()"));
    }

    fn five_slides() -> Project {
        let mut p = Project::new("Handout");
        for i in 1..=5 { p.slides.push(Slide::new(&format!("s{i}"), &format!("Slide {i}"), 1920.0, 1080.0)); }
        p
    }

    fn opts(orientation: HandoutOrientation, per_page: u8) -> HandoutOptions {
        HandoutOptions { orientation, per_page, ..Default::default() }
    }

    #[test]
    fn slides_are_grouped_onto_pages_by_how_many_go_on_a_page() {
        let p = five_slides();
        let pages = |o: &HandoutOptions| print_html_with(&p, o).matches("<section class=\"page\"").count();
        for orientation in HandoutOrientation::ALL {
            assert_eq!(pages(&opts(orientation, 1)), 5);
            assert_eq!(pages(&opts(orientation, 2)), 3, "5 slides, 2 to a page");
            assert_eq!(pages(&opts(orientation, 3)), 2, "5 slides, 3 to a page");
        }
    }

    #[test]
    fn portrait_stacks_the_slides_and_landscape_puts_them_side_by_side() {
        let p = five_slides();
        let html = |o, n| print_html_with(&p, &opts(o, n));
        assert!(html(HandoutOrientation::Portrait, 2).contains("@page{size:portrait"));
        assert!(html(HandoutOrientation::Portrait, 2).contains("grid-template-columns:repeat(1,1fr);grid-template-rows:repeat(2,1fr)"));
        assert!(html(HandoutOrientation::Landscape, 2).contains("@page{size:landscape"));
        assert!(html(HandoutOrientation::Landscape, 3).contains("grid-template-columns:repeat(3,1fr);grid-template-rows:repeat(1,1fr)"));
    }

    #[test]
    fn note_lines_go_beside_a_column_of_three_and_below_slides_that_are_stacked_or_in_columns() {
        let p = five_slides();
        let with_lines = |orientation, per_page| print_html_with(&p, &HandoutOptions { orientation, per_page, note_lines: true, ..Default::default() });
        assert_eq!(with_lines(HandoutOrientation::Portrait, 3).matches("class=\"cell beside\"").count(), 5, "three down the left, lines on the right");
        assert_eq!(with_lines(HandoutOrientation::Portrait, 2).matches("class=\"cell stack\"").count(), 5, "two stacked, lines under each");
        assert_eq!(with_lines(HandoutOrientation::Landscape, 1).matches("class=\"cell beside\"").count(), 5, "one slide with lines: lines beside it");
        assert_eq!(with_lines(HandoutOrientation::Landscape, 2).matches("class=\"cell stack\"").count(), 5, "side by side, lines under each");
        assert_eq!(with_lines(HandoutOrientation::Landscape, 3).matches("class=\"lines\"").count(), 5);
        assert!(!print_html_with(&p, &opts(HandoutOrientation::Portrait, 2)).contains("class=\"lines\""), "no lines unless asked for");
        assert!(print_html_with(&p, &opts(HandoutOrientation::Landscape, 1)).contains("cell stack center"), "a slide with nothing else is centred");
    }

    #[test]
    fn slide_notes_and_alt_text_print_only_when_asked_for() {
        let mut p = Project::new("Notes");
        let mut s = Slide::new("s1", "Intro", 1920.0, 1080.0);
        s.notes = "Say <hello> first".into();
        let mut pic = Object::new("o1", "Logo", Kind::Text, 0.0, 0.0, 100.0, 100.0);
        pic.alt = "A blue circle".into();
        let mut deco = Object::new("o2", "Swirl", Kind::Text, 0.0, 0.0, 100.0, 100.0);
        deco.alt = "A decorative swirl".into();
        deco.decorative = true;
        s.layers[0].objects = vec![pic, deco];
        p.slides.push(s);
        let html = |slide_notes, alt_text| print_html_with(&p, &HandoutOptions { slide_notes, alt_text, ..Default::default() });
        let off = html(false, false);
        assert!(!off.contains("Say &lt;hello&gt; first") && !off.contains("A blue circle"));
        let notes = html(true, false);
        assert!(notes.contains("<strong>Notes</strong> Say &lt;hello&gt; first"), "notes are escaped");
        assert!(!notes.contains("A blue circle"));
        let alt = html(false, true);
        assert!(alt.contains("<li>Logo: A blue circle</li>"), "{alt}");
        assert!(!alt.contains("A decorative swirl"), "decorative objects have no alt text to print");
    }

    #[test]
    fn a_worksheet_hides_the_correct_answers() {
        let question = |style, choices: Vec<Choice>| Question { prompt: "Pick one".into(), multiple: false, choices, points: 1.0, feedback_correct: String::new(), feedback_incorrect: String::new(), style, area: None };
        let q = question(QuestionStyle::Choice, vec![Choice { id: "a".into(), text: "Right".into(), correct: true }, Choice { id: "b".into(), text: "Wrong".into(), correct: false }]);
        let p = Project::new("Quiz");
        let with = question_html(&q, &p, true);
        assert!(with.contains("&#9745; Right") && with.contains("marks a correct answer"));
        let without = question_html(&q, &p, false);
        assert!(!without.contains("&#9745;"), "nothing is ticked: {without}");
        assert!(without.contains("&#9744; Right") && without.contains("&#9744; Wrong"));
        let t = question(QuestionStyle::TextEntry { accepted: vec!["Paris".into()], case_sensitive: false }, vec![]);
        assert!(question_html(&t, &p, true).contains("Paris"));
        assert!(!question_html(&t, &p, false).contains("Paris"));
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
