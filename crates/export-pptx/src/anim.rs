//! Animations as PowerPoint's `p:timing`: the slide's timeline becomes one group that starts by itself
//! when the slide opens, every effect carrying its delay from that moment ("with previous").
use std::collections::BTreeMap;
use std::fmt::Write as _;
use brainbend_doc::*;

struct Ids(u32);

impl Ids {
    fn next(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
}

fn tgt(spid: u32) -> String {
    format!("<p:tgtEl><p:spTgt spid=\"{spid}\"/></p:tgtEl>")
}

/// Set `style.visibility` after `delay` ms inside the effect.
fn set_visibility(ids: &mut Ids, spid: u32, visible: bool, delay: u32) -> String {
    format!(
        "<p:set><p:cBhvr><p:cTn id=\"{}\" dur=\"1\" fill=\"hold\"><p:stCondLst><p:cond delay=\"{delay}\"/></p:stCondLst></p:cTn>{}<p:attrNameLst><p:attrName>style.visibility</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal val=\"{}\"/></p:to></p:set>",
        ids.next(), tgt(spid), if visible { "visible" } else { "hidden" }
    )
}

fn anim_effect(ids: &mut Ids, spid: u32, transition: &str, filter: &str, dur: u32) -> String {
    format!("<p:animEffect transition=\"{transition}\" filter=\"{filter}\"><p:cBhvr><p:cTn id=\"{}\" dur=\"{dur}\"/>{}</p:cBhvr></p:animEffect>", ids.next(), tgt(spid))
}

/// A numeric animation of a PowerPoint attribute between two values (numbers or formulas like `#ppt_x`).
fn anim_attr(ids: &mut Ids, spid: u32, attr: &str, from: &str, to: &str, dur: u32, extra_ctn: &str, base: bool) -> String {
    let val = |v: &str| if v.parse::<f32>().is_ok() { format!("<p:fltVal val=\"{v}\"/>") } else { format!("<p:strVal val=\"{v}\"/>") };
    format!(
        "<p:anim calcmode=\"lin\" valueType=\"num\"><p:cBhvr{}><p:cTn id=\"{}\" dur=\"{dur}\" fill=\"hold\"{extra_ctn}/>{}<p:attrNameLst><p:attrName>{attr}</p:attrName></p:attrNameLst></p:cBhvr><p:tavLst><p:tav tm=\"0\"><p:val>{}</p:val></p:tav><p:tav tm=\"100000\"><p:val>{}</p:val></p:tav></p:tavLst></p:anim>",
        if base { " additive=\"base\"" } else { "" }, ids.next(), tgt(spid), val(from), val(to)
    )
}

/// One effect inside the group.
fn effect(ids: &mut Ids, preset: u32, class: &str, subtype: u32, delay: u32, repeat: Option<&str>, body: String) -> String {
    format!(
        "<p:par><p:cTn id=\"{}\" presetID=\"{preset}\" presetClass=\"{class}\" presetSubtype=\"{subtype}\"{} fill=\"hold\" nodeType=\"withEffect\"><p:stCondLst><p:cond delay=\"{delay}\"/></p:stCondLst><p:childTnLst>{body}</p:childTnLst></p:cTn></p:par>",
        ids.next(), repeat.map(|r| format!(" repeatCount=\"{r}\"")).unwrap_or_default()
    )
}

/// Entrance (`enter`) or exit of `kind` over `dur` ms. Exits run the same motions backwards and hide the object at the end.
fn transition_body(ids: &mut Ids, spid: u32, kind: EffectKind, dur: u32, enter: bool) -> (u32, u32, String) {
    let (t, io) = (if enter { "in" } else { "out" }, enter);
    let order = |a: &'static str, b: &'static str| if io { (a, b) } else { (b, a) };
    let mut s = String::new();
    if enter { s.push_str(&set_visibility(ids, spid, true, 0)); }
    let (preset, sub) = match kind {
        // PowerPoint has spin, wheel and split of its own, but a fade is the safe stand-in for them here.
        EffectKind::Fade | EffectKind::SpinQuarter | EffectKind::SpinHalf | EffectKind::SpinFull | EffectKind::Wheel
        | EffectKind::SplitVerticalOut | EffectKind::SplitVerticalIn | EffectKind::SplitHorizontalOut | EffectKind::SplitHorizontalIn => { s.push_str(&anim_effect(ids, spid, t, "fade", dur)); (10, 0) }
        EffectKind::Wipe => { s.push_str(&anim_effect(ids, spid, t, "wipe(left)", dur)); (22, 8) }
        EffectKind::Zoom => {
            for attr in ["ppt_w", "ppt_h"] {
                let (from, to) = order("0", if attr == "ppt_w" { "#ppt_w" } else { "#ppt_h" });
                s.push_str(&anim_attr(ids, spid, attr, from, to, dur, "", false));
            }
            s.push_str(&anim_effect(ids, spid, t, "fade", dur));
            (53, 16)
        }
        EffectKind::SlideLeft | EffectKind::SlideRight | EffectKind::SlideUp | EffectKind::SlideDown => {
            let (attr, off, sub) = match kind {
                EffectKind::SlideLeft => ("ppt_x", "0-#ppt_w/2", 8),
                EffectKind::SlideRight => ("ppt_x", "1+#ppt_w/2", 2),
                EffectKind::SlideUp => ("ppt_y", "0-#ppt_h/2", 1),
                _ => ("ppt_y", "1+#ppt_h/2", 4),
            };
            let home = if attr == "ppt_x" { "#ppt_x" } else { "#ppt_y" };
            let other = if attr == "ppt_x" { "ppt_y" } else { "ppt_x" };
            let (from, to) = order(off, home);
            s.push_str(&anim_attr(ids, spid, attr, from, to, dur, "", true));
            let h2 = if other == "ppt_x" { "#ppt_x" } else { "#ppt_y" };
            s.push_str(&anim_attr(ids, spid, other, h2, h2, dur, "", true));
            (2, sub)
        }
    };
    if !enter { s.push_str(&set_visibility(ids, spid, false, dur.saturating_sub(1))); }
    (preset, sub, s)
}

fn emphasis_effect(ids: &mut Ids, spid: u32, em: &Emphasis, delay: u32) -> String {
    let d = em.duration_ms.max(100);
    let repeat = if em.repeat == 0 { "indefinite".to_string() } else { (em.repeat * 1000).to_string() };
    let rep = Some(repeat.as_str());
    let tgt_el = tgt(spid);
    match em.kind {
        EmphasisKind::Spin => {
            let b = format!("<p:animRot by=\"21600000\"><p:cBhvr><p:cTn id=\"{}\" dur=\"{d}\" fill=\"hold\"/>{tgt_el}<p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot>", ids.next());
            effect(ids, 8, "emph", 0, delay, rep, b)
        }
        EmphasisKind::Pulse | EmphasisKind::Grow => {
            let (scale, rev, preset) = if em.kind == EmphasisKind::Pulse { (112000, " autoRev=\"1\"", 26) } else { (130000, "", 6) };
            let half = if rev.is_empty() { d } else { (d / 2).max(50) };
            let b = format!("<p:animScale><p:cBhvr><p:cTn id=\"{}\" dur=\"{half}\"{rev} fill=\"hold\"/>{tgt_el}</p:cBhvr><p:by x=\"{scale}\" y=\"{scale}\"/></p:animScale>", ids.next());
            effect(ids, preset, "emph", 0, delay, if em.kind == EmphasisKind::Grow { None } else { rep }, b)
        }
        EmphasisKind::Swing => {
            let b = format!("<p:animRot by=\"840000\"><p:cBhvr><p:cTn id=\"{}\" dur=\"{}\" autoRev=\"1\" fill=\"hold\"/>{tgt_el}<p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot>", ids.next(), (d / 2).max(50));
            effect(ids, 32, "emph", 0, delay, rep, b)
        }
        EmphasisKind::Shake => {
            let cycles = if em.repeat == 0 { "indefinite".to_string() } else { (em.repeat * 3000).to_string() };
            let b = anim_attr(ids, spid, "ppt_x", "#ppt_x", "#ppt_x+0.012", (d / 6).max(30), &format!(" autoRev=\"1\" repeatCount=\"{cycles}\""), true);
            effect(ids, 32, "emph", 0, delay, None, b)
        }
        EmphasisKind::Flash => {
            let b = anim_attr(ids, spid, "style.opacity", "1", "0.15", (d / 2).max(50), " autoRev=\"1\"", false);
            effect(ids, 35, "emph", 0, delay, rep, b)
        }
        EmphasisKind::Bounce => {
            let b = anim_attr(ids, spid, "ppt_y", "#ppt_y", "#ppt_y-0.04", (d / 2).max(50), " autoRev=\"1\"", true);
            effect(ids, 6, "emph", 0, delay, rep, b)
        }
    }
}

/// The path as PowerPoint writes it: fractions of the slide, curves as cubic Béziers.
fn path_string(m: &MotionPath, sw: f32, sh: f32) -> String {
    let p = &m.points;
    let pt = |q: (f32, f32)| format!("{:.5} {:.5}", q.0 / sw, q.1 / sh);
    let mut s = format!("M {}", pt(p[0]));
    for i in 0..p.len() - 1 {
        if m.smooth && p.len() >= 3 {
            let (p0, p1, p2, p3) = (p[i.saturating_sub(1)], p[i], p[i + 1], p[(i + 2).min(p.len() - 1)]);
            let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
            let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
            let _ = write!(s, " C {} {} {}", pt(c1), pt(c2), pt(p2));
        } else {
            let _ = write!(s, " L {}", pt(p[i + 1]));
        }
    }
    s.push_str(" E");
    s
}

fn motion_effect(ids: &mut Ids, spid: u32, m: &MotionPath, delay: u32, sw: f32, sh: f32) -> String {
    let b = format!(
        "<p:animMotion origin=\"layout\" path=\"{}\" pathEditMode=\"relative\" rAng=\"0\"><p:cBhvr><p:cTn id=\"{}\" dur=\"{}\" fill=\"hold\"/>{}<p:attrNameLst><p:attrName>ppt_x</p:attrName><p:attrName>ppt_y</p:attrName></p:attrNameLst></p:cBhvr></p:animMotion>",
        path_string(m, sw, sh), ids.next(), m.duration_ms.max(100), tgt(spid)
    );
    effect(ids, 0, "path", 0, delay, None, b)
}

/// The slide's `p:timing`, or `None` when no object has an animation or a timed appearance.
/// Effects that start by themselves go in one group that begins when the slide opens (each with its delay
/// from that moment); effects that wait for a click go in one group per click number, in order.
/// `spids` maps object ids to the shape ids written in the slide; `layer_ms` is the base layer's length.
pub fn timing_xml(objects: &[Object], layer_ms: Option<u32>, spids: &BTreeMap<String, u32>, sw: f32, sh: f32) -> Option<String> {
    let mut ids = Ids(4); // 1-4 are the wrapper nodes below
    // (click number, delay, xml); click number 0 = automatic.
    let mut effects: Vec<(u32, u32, String)> = vec![];
    for o in objects {
        let Some(&spid) = spids.get(&o.id) else { continue };
        let cs = o.click_steps;
        let start = o.timing.map(|t| t.start_ms).unwrap_or(0);
        let end = match o.timing {
            Some(Timing { start_ms, dur_ms: Some(d) }) => Some(start_ms + d),
            _ if o.exit.is_some() => layer_ms,
            _ => None,
        };
        if let Some(e) = o.entrance {
            let (preset, sub, body) = transition_body(&mut ids, spid, e.kind, e.duration_ms.max(1), true);
            let delay = if cs.entrance > 0 { e.delay_ms } else { start + e.delay_ms };
            effects.push((cs.entrance, delay, effect(&mut ids, preset, "entr", sub, delay, None, body)));
        } else if start > 0 {
            let body = set_visibility(&mut ids, spid, true, 0);
            effects.push((0, start, effect(&mut ids, 1, "entr", 0, start, None, body)));
        }
        if let Some(em) = o.emphasis {
            let delay = if cs.emphasis > 0 { em.delay_ms } else { start + em.delay_ms };
            effects.push((cs.emphasis, delay, emphasis_effect(&mut ids, spid, &em, delay)));
        }
        if let Some(m) = o.motion.as_ref().filter(|m| m.points.len() >= 2) {
            let delay = if cs.motion > 0 { m.delay_ms } else { start + m.delay_ms };
            effects.push((cs.motion, delay, motion_effect(&mut ids, spid, m, delay, sw, sh)));
        }
        if cs.exit > 0 {
            if let Some(x) = o.exit {
                let (preset, sub, body) = transition_body(&mut ids, spid, x.kind, x.duration_ms.max(1), false);
                effects.push((cs.exit, x.delay_ms, effect(&mut ids, preset, "exit", sub, x.delay_ms, None, body)));
            }
        } else if let Some(end) = end {
            if let Some(x) = o.exit {
                let dur = x.duration_ms.max(1);
                let (preset, sub, body) = transition_body(&mut ids, spid, x.kind, dur, false);
                let delay = end.saturating_sub(dur);
                effects.push((0, delay, effect(&mut ids, preset, "exit", sub, delay, None, body)));
            } else {
                let body = set_visibility(&mut ids, spid, false, 0);
                effects.push((0, end, effect(&mut ids, 1, "exit", 0, end, None, body)));
            }
        }
    }
    if effects.is_empty() { return None; }
    effects.sort_by_key(|(step, delay, _)| (*step, *delay));
    let mut steps: Vec<u32> = effects.iter().map(|e| e.0).collect();
    steps.dedup();
    let mut groups = String::new();
    for step in steps {
        let mut inner = String::new();
        for (i, (_, _, x)) in effects.iter().filter(|e| e.0 == step).enumerate() {
            // The first effect of a click group is the one the click starts; the rest go with it.
            inner.push_str(&if step > 0 && i == 0 { x.replacen("nodeType=\"withEffect\"", "nodeType=\"clickEffect\"", 1) } else { x.clone() });
        }
        let (outer_id, inner_id) = if step == 0 { (3, 4) } else { (ids.next(), ids.next()) };
        let start_cond = if step == 0 { "<p:cond delay=\"indefinite\"/><p:cond evt=\"onBegin\" delay=\"0\"><p:tn val=\"2\"/></p:cond>" } else { "<p:cond delay=\"indefinite\"/>" };
        let _ = write!(groups, "<p:par><p:cTn id=\"{outer_id}\" fill=\"hold\"><p:stCondLst>{start_cond}</p:stCondLst><p:childTnLst><p:par><p:cTn id=\"{inner_id}\" fill=\"hold\"><p:stCondLst><p:cond delay=\"0\"/></p:stCondLst><p:childTnLst>{inner}</p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par>");
    }
    Some(format!(
        "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst>\
<p:seq concurrent=\"1\" nextAc=\"seek\"><p:cTn id=\"2\" dur=\"indefinite\" nodeType=\"mainSeq\"><p:childTnLst>{groups}</p:childTnLst></p:cTn>\
<p:prevCondLst><p:cond evt=\"onPrev\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst><p:nextCondLst><p:cond evt=\"onNext\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst></p:seq>\
</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"
    ))
}
