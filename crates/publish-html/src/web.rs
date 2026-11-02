//! Publishing a Web Lesson: one responsive page (`index.html`) plus its media.
use crate::*;
use brainbend_doc::Rgba;

const WEB_RUNTIME: &str = include_str!("web_runtime.js");
const WEB_CSS: &str = include_str!("web.css");
const WEB_PAGE: &str = r##"<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>%TITLE%</title>
<style>
@font-face{font-family:Montserrat;src:url(fonts/Montserrat.ttf);font-weight:100 900}
@font-face{font-family:"Open Sans";src:url(fonts/OpenSans.ttf);font-weight:300 800;font-stretch:75% 100%}
:root{%VARS%}
%CSS%
%QCSS%
</style></head>
<body>
<a class="skip" href="#main">Skip to the lesson</a>
<div id="home" class="wl-home">
<header class="wl-hero"><div class="wl-hero-in">
<div class="wl-hero-text">
<p class="wl-eyebrow" id="hero-sub" hidden></p>
<h1 id="hero-title" tabindex="-1"></h1>
<p class="wl-hero-desc" id="hero-desc" hidden></p>
<div class="wl-hero-actions"><button id="start" type="button" class="wl-btn wl-btn-light">Start</button><span class="wl-hero-meta" id="hero-meta"></span></div>
<div class="wl-hero-prog"><div class="wl-bar-track" id="home-bar-box" role="progressbar" aria-label="Lesson progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0"><div id="home-bar"></div></div><span id="home-bar-text">0%</span></div>
</div>
<img id="hero-img" alt="" hidden>
</div></header>
<section class="wl-modules" id="modules" tabindex="-1" aria-labelledby="mods-h">
<div id="done" class="wl-done" hidden role="status"><h2>Lesson complete</h2><p class="wl-done-score"></p></div>
<h2 id="mods-h">Modules</h2>
<ol id="cards" class="wl-cards"></ol>
</section>
</div>
<div id="module" class="wl-module" hidden>
<header class="wl-top">
<button id="back-btn" type="button" class="wl-ghost"><span aria-hidden="true">&#8249;</span> Overview</button>
<div class="wl-title">%TITLE%</div>
<div class="wl-bar"><div class="wl-bar-track" id="bar-box" role="progressbar" aria-label="Module progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0"><div id="bar"></div></div><span id="bar-text">0%</span></div>
<button id="menu-btn" type="button" class="wl-ghost" aria-expanded="false" aria-controls="menu">Modules</button>
</header>
<div class="wl-layout">
<nav id="menu" aria-label="Modules"><button id="menu-home" type="button" class="wl-ghost"><span aria-hidden="true">&#8249;</span> All modules</button><ol></ol></nav>
<div id="scrim" hidden></div>
<main id="main" tabindex="-1"></main>
</div>
</div>
<script>window.__BRAINBEND__=%JSON%;</script>
<script>%RUNTIME%</script>
</body></html>
"##;

fn hex(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

use brainbend_doc::check::contrast_ratio;

const BLACK: Rgba = Rgba(0, 0, 0, 255);
const WHITE: Rgba = Rgba(255, 255, 255, 255);
const INK: Rgba = Rgba(0x14, 0x18, 0x1f, 255);

/// `a` and `b` blended in sRGB, `t` of `a` (what CSS `color-mix(in srgb, a t%, b)` does).
fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let m = |x: u8, y: u8| (x as f32 * t + y as f32 * (1.0 - t)).round().clamp(0.0, 255.0) as u8;
    Rgba(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2), 255)
}

fn worst(c: Rgba, bgs: &[Rgba]) -> f32 {
    bgs.iter().map(|b| contrast_ratio(c, *b)).fold(f32::MAX, f32::min)
}

/// White or near-black, whichever reads better on every one of `bgs`.
fn on_all(bgs: &[Rgba]) -> Rgba {
    if worst(WHITE, bgs) >= worst(INK, bgs) { WHITE } else { INK }
}

/// `bg` itself when white or ink text reaches AA on it; else the nearest shade that does, with that text colour.
fn fit_fill(bg: Rgba) -> (Rgba, Rgba) {
    let fg = on_all(&[bg]);
    if worst(fg, &[bg]) >= 4.5 {
        return (bg, fg);
    }
    let darker = (1..=20).map(|i| mix(BLACK, bg, i as f32 / 40.0)).find(|c| worst(WHITE, &[*c]) >= 4.5).map(|c| (c, WHITE));
    let lighter = (1..=20).map(|i| mix(WHITE, bg, i as f32 / 40.0)).find(|c| worst(INK, &[*c]) >= 4.5).map(|c| (c, INK));
    // The smaller change keeps the brand colour recognisable.
    let dist = |c: Rgba| (c.0 as i32 - bg.0 as i32).abs() + (c.1 as i32 - bg.1 as i32).abs() + (c.2 as i32 - bg.2 as i32).abs();
    match (darker, lighter) {
        (Some(d), Some(l)) => if dist(d.0) <= dist(l.0) { d } else { l },
        (Some(d), None) => d,
        (None, Some(l)) => l,
        (None, None) => (bg, fg),
    }
}

fn on(bg: Rgba) -> &'static str {
    if on_all(&[bg]) == WHITE { "#ffffff" } else { "#14181f" }
}

/// `fg` itself when it reads on all of `bgs`; else the nearest shade of it that does (WCAG AA, `min`:1).
fn readable_on(fg: Rgba, bgs: &[Rgba], min: f32) -> Rgba {
    if worst(fg, bgs) >= min {
        return fg;
    }
    let toward = if worst(BLACK, bgs) >= worst(WHITE, bgs) { BLACK } else { WHITE };
    (1..=20).map(|i| mix(toward, fg, i as f32 / 20.0)).find(|c| worst(*c, bgs) >= min).unwrap_or(toward)
}

fn quote_font(f: &str) -> String {
    format!("\"{}\"", f.replace(['"', '\\', ';', '{', '}', '<', '>'], ""))
}

pub(crate) fn build_web(project: &Project, lms: Lms) -> Result<Vec<(String, Vec<u8>)>, String> {
    let lesson = project.lesson.as_ref().ok_or("project has no lesson content")?;
    if lesson.sections.is_empty() {
        return Err("the lesson has no sections".into());
    }
    let mut files = base_files();
    let urls = package_assets(project, &mut files);
    let t = &project.theme;
    // The lesson's own primary and secondary colours win over the theme's.
    let primary = lesson.primary.unwrap_or(t.color(4));
    let secondary = lesson.secondary.unwrap_or(t.color(5));
    // Colours that carry text are nudged until they pass WCAG AA on the surfaces they sit on.
    let (text, page, dark) = (t.color(0), t.color(1), t.color(2));
    let soft = mix(text, page, 0.05);
    let hero_end = mix(dark, primary, 0.72);
    let on_dark = on_all(&[dark, hero_end]);
    let accent_text = readable_on(primary, &[page, soft], 4.5);
    let (accent_fill, on_accent) = fit_fill(primary);
    let eyebrow = readable_on(mix(secondary, WHITE, 0.7), &[dark, hero_end], 4.5);
    let muted = (68..=100)
        .map(|p| mix(text, page, p as f32 / 100.0))
        .find(|c| worst(*c, &[page, soft]) >= 4.5)
        .unwrap_or(text);
    let vars = format!(
        "--text:{};--page:{};--dark:{};--on-dark:{};--light:{};--accent:{};--accent-fill:{};--on-accent:{};--accent2:{};--on-accent2:{};--accent-text:{};--eyebrow:{};--muted:{};--heading-font:{};--body-font:{}",
        hex(text), hex(page), hex(dark), hex(on_dark), hex(t.color(3)), hex(primary), hex(accent_fill), hex(on_accent), hex(secondary), on(secondary),
        hex(accent_text), hex(eyebrow), hex(muted), quote_font(&t.heading_font), quote_font(&t.body_font)
    );
    let mut value = serde_json::json!({
        "name": project.name,
        "lesson": lesson,
        "pass_mark": project.pass_mark,
        "variables": project.variables,
        "assetUrls": urls,
        "lms": lms.player_mode(),
    });
    // The question styles and the LMS layer read the same fields the slide player does.
    value["slides"] = serde_json::json!([]);
    let json = serde_json::to_string(&value).map_err(|e| e.to_string())?.replace('<', "\\u003c");
    let html = WEB_PAGE
        .replace("%TITLE%", &escape_html(&project.name))
        .replace("%VARS%", &vars)
        .replace("%CSS%", WEB_CSS)
        .replace("%QCSS%", QUESTION_CSS)
        .replace("%RUNTIME%", &shared(WEB_RUNTIME))
        .replace("%JSON%", &json);
    files.push(("index.html".into(), html.into_bytes()));
    if lms != Lms::None {
        let manifest = manifest(project, lms, &files);
        files.push(("imsmanifest.xml".into(), manifest.into_bytes()));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_on_an_accent_picks_whichever_of_white_and_ink_reads_better() {
        // A mid-tone where white (3.5:1) loses to ink (5.1:1): the old luminance cut-off picked white.
        let mid = Rgba(0x8a, 0x8a, 0x8a, 255);
        assert_eq!(on(mid), "#14181f");
        assert_eq!(on(Rgba(0x03, 0x53, 0x9e, 255)), "#ffffff");
        assert_eq!(on(Rgba(0xff, 0xe0, 0x4a, 255)), "#14181f");
    }

    #[test]
    fn a_fill_that_neither_white_nor_ink_reads_on_is_shifted_until_one_does() {
        // Sunset's coral: white gives 4.35:1 and ink 4.2:1, so the fill moves a little.
        let coral = Rgba(0xd6, 0x45, 0x50, 255);
        let (fill, text) = fit_fill(coral);
        assert!(contrast_ratio(text, fill) >= 4.5, "{fill:?} {text:?}");
        assert!((fill.0 as i32 - coral.0 as i32).abs() < 40, "still recognisably the same colour: {fill:?}");
        // A colour that already works is untouched.
        let navy = Rgba(0x03, 0x53, 0x9e, 255);
        assert_eq!(fit_fill(navy), (navy, WHITE));
        // Every colour on a coarse grid ends up readable.
        for r in (0..=255).step_by(51) { for g in (0..=255).step_by(51) { for b in (0..=255).step_by(51) {
            let (f, t) = fit_fill(Rgba(r as u8, g as u8, b as u8, 255));
            assert!(contrast_ratio(t, f) >= 4.5, "{r},{g},{b} -> {f:?} {t:?}");
        } } }
    }

    #[test]
    fn accent_coloured_text_is_darkened_until_it_passes() {
        let page = Rgba(255, 255, 255, 255);
        let pale = Rgba(0xf2, 0xa9, 0x00, 255); // an orange that fails as text on white
        assert!(contrast_ratio(pale, page) < 4.5);
        let fixed = readable_on(pale, &[page], 4.5);
        assert!(contrast_ratio(fixed, page) >= 4.5);
        // A colour that already passes is left alone.
        let navy = Rgba(0x03, 0x53, 0x9e, 255);
        assert_eq!(readable_on(navy, &[page], 4.5), navy);
        // On a dark page it is lightened instead.
        let dark = Rgba(0x10, 0x14, 0x1c, 255);
        assert!(contrast_ratio(readable_on(navy, &[dark], 4.5), dark) >= 4.5);
    }

    #[test]
    fn a_published_lesson_carries_readable_text_colours() {
        let mut p = Project::new_web_lesson("t", brainbend_doc::ProjectKind::WebLesson);
        p.sanitize();
        let files = crate::build(&p, Lms::None).unwrap();
        let html = String::from_utf8(files.iter().find(|(n, _)| n == "index.html").unwrap().1.clone()).unwrap();
        for var in ["--accent-text", "--eyebrow", "--muted", "--on-dark"] {
            assert!(html.contains(&format!("{var}:#")), "{var} missing");
        }
    }
}
