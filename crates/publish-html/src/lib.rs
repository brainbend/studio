//! Static HTML5 export. Writes `index.html` (data embedded) plus an `assets/` folder.
//! The player (`runtime.js`) is original BrainBend Studio code.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use brainbend_doc::Project;
mod print;
pub use print::{print_html, print_html_with, HandoutOptions, HandoutOrientation};

mod web;

const QUESTION_CSS: &str = include_str!("question.css");
const RUNTIME_SRC: &str = include_str!("runtime.js");
const LMS_JS: &str = include_str!("shared_lms.js");
const QUESTIONS_JS: &str = include_str!("shared_questions.js");
/// Which LMS packaging the player should expect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lms {
    None,
    Scorm12,
    Scorm2004,
}

impl Lms {
    fn player_mode(self) -> &'static str {
        match self {
            Lms::None => "none",
            Lms::Scorm12 => "1.2",
            Lms::Scorm2004 => "2004",
        }
    }
}

/// Build every file of the export in memory: (relative path, bytes).
/// The fonts a lesson needs: every family it names, plus the two the player falls back to.
fn fonts_needed(project: &Project) -> std::collections::BTreeSet<String> {
    let mut used = project.fonts_used();
    used.insert("Open Sans".into());
    used.insert("Montserrat".into());
    used
}

/// The font files an export carries and the `@font-face` rules that load them.
fn font_assets(project: &Project) -> (Vec<(String, Vec<u8>)>, String) {
    let e = brainbend_fonts::embed(&fonts_needed(project), "fonts/");
    (e.files, e.css)
}

/// Which of the fonts a lesson names travel with it, and which do not (the learner's own copy is used if they
/// have one). For the Publish dialog.
#[derive(Clone, Default)]
pub struct FontsReport {
    pub embedded: Vec<String>,
    pub fallback: Vec<String>,
}

pub fn fonts_report(project: &Project) -> FontsReport {
    let e = brainbend_fonts::embed(&project.fonts_used(), "fonts/");
    FontsReport { embedded: e.embedded, fallback: e.fallback }
}

/// Add the media something references to `files` and return asset key -> file path.
/// (Deleted images do not ship.)
fn package_assets(project: &Project, files: &mut Vec<(String, Vec<u8>)>) -> BTreeMap<String, String> {
    let mut urls = BTreeMap::new();
    let used = project.referenced_assets();
    for (i, (key, asset)) in project.assets.iter().filter(|(k, _)| used.contains(k.as_str())).enumerate() {
        let ext = match asset.mime.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            "image/svg+xml" => "svg",
            "video/mp4" => "mp4",
            "video/webm" => "webm",
            "audio/mpeg" => "mp3",
            "audio/mp4" => "m4a",
            "audio/wav" => "wav",
            "text/vtt" => "vtt",
            _ => "bin",
        };
        let rel = format!("assets/a{i}.{ext}");
        files.push((rel.clone(), asset.bytes.clone()));
        urls.insert(key.clone(), rel);
    }
    urls
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// 360° pictures ship as a script that sets a `data:` address, not as a picture file. A page opened straight from disk
/// (the preview) may not let WebGL read a picture file, but it can read one that arrived as a script.
fn package_panos(project: &Project, files: &mut Vec<(String, Vec<u8>)>, urls: &mut BTreeMap<String, String>) -> String {
    let mut keys: Vec<String> = Vec::new();
    for slide in &project.flattened().slides {
        for layer in &slide.layers {
            for o in &layer.objects {
                if let (brainbend_doc::Kind::Pano(_), Some(a)) = (&o.kind, &o.asset) {
                    if !keys.contains(a) { keys.push(a.clone()); }
                }
            }
        }
    }
    let mut tags = String::new();
    for (i, key) in keys.iter().enumerate() {
        let Some(asset) = project.assets.get(key) else { continue };
        if let Some(rel) = urls.get(key).cloned() { files.retain(|(n, _)| *n != rel); }
        let rel = format!("assets/pano{i}.js");
        let js = format!("(window.__panoPix=window.__panoPix||{{}})[{}]=\"data:{};base64,{}\";", serde_json::to_string(key).unwrap_or_default(), asset.mime, base64(&asset.bytes));
        files.push((rel.clone(), js.into_bytes()));
        tags.push_str(&format!("<script src=\"{rel}\"></script>\n"));
        urls.insert(key.clone(), rel);
    }
    tags
}

pub fn build(project: &Project, lms: Lms) -> Result<Vec<(String, Vec<u8>)>, String> {
    if project.is_web() {
        return web::build_web(project, lms);
    }
    if project.slides.is_empty() {
        return Err("project has no slides".into());
    }
    let (mut files, font_css) = font_assets(project);
    let mut urls = package_assets(project, &mut files);
    let pano_tags = package_panos(project, &mut files, &mut urls);

    let mut data = project.flattened();
    data.assets.clear();
    let mut value = serde_json::to_value(&data).map_err(|e| e.to_string())?;
    // A picture cropped to a shape gets its clip-path ready made, so the player needs no geometry.
    if let Some(slides) = value.get_mut("slides").and_then(|s| s.as_array_mut()) {
        for (js, slide) in slides.iter_mut().zip(&data.slides) {
            let layers = js.get_mut("layers").and_then(|l| l.as_array_mut());
            for (jl, layer) in layers.into_iter().flatten().zip(&slide.layers) {
                let objs = jl.get_mut("objects").and_then(|o| o.as_array_mut());
                for (jo, o) in objs.into_iter().flatten().zip(&layer.objects) {
                    if let (Some(shape), Some(map)) = (o.mask, jo.as_object_mut()) {
                        map.insert("mask_clip".into(), serde_json::Value::String(shape.css_polygon(o.w, o.h)));
                    }
                }
            }
        }
    }
    value["assetUrls"] = serde_json::to_value(&urls).map_err(|e| e.to_string())?;
    value["lms"] = serde_json::Value::String(lms.player_mode().into());
    // `<` is escaped so embedded text can never close the script element.
    let json = serde_json::to_string(&value).map_err(|e| e.to_string())?.replace('<', "\\u003c");

    let html = format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title}</title>
<style>
:root{{{pvars}}}
{font_css}html,body{{margin:0;height:100%;background:var(--p-body);overflow:hidden}}
#stage{{position:absolute;left:50%;top:50%;transform-origin:center;background:#fff;overflow:hidden}}
.slide,.layer{{position:absolute;inset:0}}
.q{{position:absolute;inset:0;box-sizing:border-box;padding:110px 180px;background:#f4f6f9;font-family:"Open Sans",Arial,sans-serif;color:#1b2a3a;overflow:auto}}
{qcss}
.r-score{{font-size:200px;font-weight:800;color:#03539E;line-height:1.1}}
.r-detail{{font-size:42px;margin:12px 0}}
.r-verdict{{font-size:52px;font-weight:700;margin-bottom:30px}}
#progress{{position:absolute;left:0;right:0;top:0;height:4px;background:var(--p-line)}}
#progress div{{height:100%;width:0;background:var(--p-progress);transition:width .3s}}
#menu{{position:fixed;left:0;top:0;bottom:48px;width:320px;max-width:85vw;overflow:auto;background:var(--p-panel);color:var(--p-fg);font:calc(16px*var(--p-scale)) var(--p-font);z-index:5;box-shadow:4px 0 16px rgba(0,0,0,.4)}}
#menu ol{{list-style:none;margin:0;padding:12px}}
#menu button{{display:block;width:100%;text-align:left;font:inherit;color:inherit;background:transparent;border:0;border-radius:8px;padding:12px 14px;cursor:pointer}}
#menu button:hover:not(:disabled){{background:var(--p-hover)}}
#menu button.visited::before{{content:"\2713 ";color:var(--p-progress)}}
#menu button[aria-current=page]{{background:var(--p-accent);color:var(--p-accent-ink)}}
#menu button:disabled{{color:var(--p-muted);cursor:default}}
#menu button:focus-visible{{outline:3px solid #6aa8e6}}
#topbar{{position:fixed;left:0;right:0;top:0;height:44px;box-sizing:border-box;display:flex;align-items:center;gap:12px;padding:0 12px;background:var(--p-panel);color:var(--p-fg);font:calc(16px*var(--p-scale)) var(--p-font);z-index:4;border-bottom:1px solid var(--p-line)}}
#topbar .tb-toggle{{display:flex;align-items:center;justify-content:center;width:32px;height:32px;padding:0;border:0;border-radius:6px;background:transparent;color:inherit;cursor:pointer}}
#topbar .tb-toggle:hover{{background:var(--p-hover)}}
#topbar .tb-title{{font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}
#topbar .logo{{height:28px;max-width:140px;object-fit:contain}}
body.side-anim #topbar,body.side-anim #nav{{transition:left .22s ease,right .22s ease}}
body.side-anim #stage{{transition:transform .22s ease,left .22s ease}}
#menu.sidebar{{width:280px;box-shadow:none;z-index:3}}
#menu.side-left{{border-right:1px solid var(--p-line)}}
#menu.side-right{{left:auto;right:0;border-left:1px solid var(--p-line)}}
#menu .banner{{display:block;width:100%;height:120px;object-fit:cover}}
#menu .sb-tabs{{display:flex;border-bottom:1px solid var(--p-line);margin-top:4px}}
#menu .sb-tabs button{{flex:1;border-radius:0;text-align:center;padding:10px 8px;border-bottom:3px solid transparent;color:var(--p-muted)}}
#menu .sb-tabs button[aria-selected=true]{{color:var(--p-fg);border-bottom-color:var(--p-accent);background:transparent}}
#menu .notes{{padding:14px 16px;line-height:1.5}}
#menu .notes-text{{white-space:pre-wrap}}
#menu .notes-empty{{color:var(--p-muted)}}
#menu dl.notes dt{{font-weight:600;margin-top:10px}}
#menu dl.notes dd{{margin:2px 0 0;color:var(--p-muted)}}
#menu .sb-head{{display:flex;align-items:stretch;border-bottom:1px solid var(--p-line);margin-top:4px}}
#menu .sb-head .sb-tabs{{flex:1;border-bottom:0;margin-top:0}}
#menu .sb-head .sb-search{{flex:0 0 40px;display:flex;align-items:center;justify-content:center;padding:0;color:var(--p-muted)}}
#menu .sb-searchbox{{padding:8px 12px;border-bottom:1px solid var(--p-line)}}
#menu .sb-searchbox input{{width:100%;box-sizing:border-box;font:inherit;padding:8px 10px;border-radius:6px;border:1px solid var(--p-line);background:var(--p-body);color:var(--p-fg)}}
#menu.sidebar ol{{padding:4px 0}}
#menu.sidebar li.nomatch,#menu.sidebar li.collapsed{{display:none}}
#menu.sidebar li.scene-head button{{font-weight:700;border-radius:0;padding:14px 16px}}
#menu.sidebar li.scene-head button::before{{content:"";display:inline-block;margin-right:10px;border:5px solid transparent;border-top-color:currentColor;border-bottom-width:0;vertical-align:middle;transition:transform .15s}}
#menu.sidebar li.scene-head button[aria-expanded=false]::before{{transform:rotate(-90deg)}}
#menu.sidebar li:not(.scene-head) button{{border-radius:0;padding:12px 16px 12px 24px;border-left:4px solid transparent;position:relative}}
#menu.sidebar li button[aria-current=page]{{background:var(--p-hover);color:var(--p-fg);border-left-color:var(--p-progress)}}
#menu.sidebar li button.visited::before{{content:""}}
#menu.sidebar li button.visited::after{{content:"\2713";position:absolute;right:16px;top:50%;transform:translateY(-50%);color:var(--p-fg)}}
#topbar .tb-resources{{margin-left:auto;font:inherit;font-weight:700;font-size:.8em;letter-spacing:.06em;text-transform:uppercase;color:inherit;background:transparent;border:0;padding:8px;cursor:pointer}}
#resources{{position:fixed;top:44px;right:8px;z-index:6;min-width:200px;max-width:80vw;background:var(--p-panel);color:var(--p-fg);border:1px solid var(--p-line);border-radius:8px;box-shadow:0 8px 24px rgba(0,0,0,.4);font:calc(16px*var(--p-scale)) var(--p-font)}}
#resources ul{{list-style:none;margin:0;padding:8px 0}}
#resources a{{display:block;padding:8px 16px;color:var(--p-progress);text-decoration:none}}
#resources a:hover{{background:var(--p-hover)}}
#nav{{position:fixed;left:0;right:0;bottom:0;height:48px;display:flex;align-items:center;justify-content:flex-end;gap:20px;padding:0 12px;background:var(--p-panel);color:var(--p-fg);font:calc(16px*var(--p-scale)) var(--p-font)}}
#nav button{{font:inherit;padding:8px 22px;border-radius:8px;border:0;background:var(--p-accent);color:var(--p-accent-ink);cursor:pointer}}
#nav button:disabled{{background:var(--p-disabled);cursor:default}}
#nav button.has-icon{{display:inline-flex;align-items:center;gap:8px;padding:8px 14px}}
#nav .nav-controls{{order:-1;margin-right:auto;display:flex;align-items:center;gap:8px}}
#nav .nav-controls button{{background:transparent;color:var(--p-fg);border:1px solid var(--p-line);padding:6px 10px;min-width:36px}}
#nav .nav-controls button:hover{{background:var(--p-hover)}}
#nav .nav-controls button[aria-pressed=true]{{background:var(--p-accent);color:var(--p-accent-ink);border-color:var(--p-accent)}}
#nav .nav-controls button:disabled{{opacity:.4;cursor:default}}
#nav .nav-controls .ctl-seek{{width:200px;max-width:28vw;accent-color:var(--p-accent)}}
#nav .nav-controls .ctl-time{{font-size:.85em;color:var(--p-muted);white-space:nowrap;font-variant-numeric:tabular-nums}}
#nav .nav-controls .ctl-volume[aria-pressed=true]{{background:var(--p-hover);color:var(--p-fg);border-color:var(--p-line)}}
#nav button:focus-visible{{outline:3px solid #6aa8e6}}
.obj{{position:absolute;box-sizing:border-box;user-select:none;pointer-events:auto}}.hidden-by-layer{{display:none !important}}
.obj:focus-visible{{outline:3px solid #1a1a1a;outline-offset:3px}}
#slide-title:focus{{outline:0}}
@media (forced-colors:active){{.obj:focus-visible,#nav button:focus-visible,#menu button:focus-visible{{outline-color:Highlight}}}}
.obj img,.obj video{{position:absolute;inset:0;width:100%;height:100%}}
.obj audio{{position:absolute;left:0;right:0;top:50%;transform:translateY(-50%);width:100%}}
.obj.media .label{{pointer-events:none}}
.marker-card.mk-show{{animation:mk-in .18s ease-out}}
@keyframes mk-in{{from{{opacity:0;transform:scale(.88) translateY(6px)}}to{{opacity:1;transform:none}}}}
@media (prefers-reduced-motion:reduce){{.marker-card.mk-show{{animation:none}}}}
.obj .label{{margin:0;position:absolute;inset:0;box-sizing:border-box;display:flex;flex-direction:column;justify-content:center;white-space:pre-wrap}}
</style></head>
<body><main id="stage" aria-label="{title}"></main>
{pano_tags}<script>window.__BRAINBEND__={json};</script>
<script>{runtime}</script>
</body></html>
"#,
        title = escape_html(&project.name),
        pvars = player_vars(&project.player),
        json = json,
        runtime = shared(RUNTIME_SRC),
        pano_tags = pano_tags,
        qcss = QUESTION_CSS
    );
    files.push(("index.html".into(), html.into_bytes()));
    if lms != Lms::None {
        let manifest = manifest(project, lms, &files);
        files.push(("imsmanifest.xml".into(), manifest.into_bytes()));
    }
    Ok(files)
}

/// Write a plain (non-LMS) web export into `out_dir`.
pub fn export(project: &Project, out_dir: &Path) -> Result<(), String> {
    for (rel, bytes) in build(project, Lms::None)? {
        let path = out_dir.join(&rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Write a SCORM package (ZIP with `imsmanifest.xml` at its root) to `zip_path`.
pub fn export_scorm(project: &Project, version: Lms, zip_path: &Path) -> Result<(), String> {
    use std::io::Write;
    if version == Lms::None {
        return Err("choose a SCORM version".into());
    }
    let files = build(project, version)?;
    let tmp = zip_path.with_extension("zip.tmp");
    {
        let file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        // Manifest first, as many LMS importers expect.
        let mut ordered: Vec<&(String, Vec<u8>)> = files.iter().filter(|(p, _)| p == "imsmanifest.xml").collect();
        ordered.extend(files.iter().filter(|(p, _)| p != "imsmanifest.xml"));
        for (path, bytes) in ordered {
            zip.start_file(path.as_str(), opts).map_err(|e| e.to_string())?;
            zip.write_all(bytes).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, zip_path).map_err(|e| e.to_string())
}

/// Single-SCO manifest: the whole course is one launchable resource.
fn manifest(project: &Project, lms: Lms, files: &[(String, Vec<u8>)]) -> String {
    let title = escape_html(&project.name);
    let file_list: String = files
        .iter()
        .filter(|(p, _)| p != "imsmanifest.xml")
        .map(|(p, _)| format!("      <file href=\"{}\"/>\n", escape_html(p)))
        .collect();
    // Identifiers only need to be unique and stable per course name.
    let id: String = project.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let id = format!("brainbend_{id}");
    match lms {
        Lms::Scorm12 => format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest identifier="{id}" version="1" xmlns="http://www.imsproject.org/xsd/imscp_rootv1p1p2" xmlns:adlcp="http://www.adlnet.org/xsd/adlcp_rootv1p2" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.imsproject.org/xsd/imscp_rootv1p1p2 imscp_rootv1p1p2.xsd http://www.adlnet.org/xsd/adlcp_rootv1p2 adlcp_rootv1p2.xsd">
  <metadata>
    <schema>ADL SCORM</schema>
    <schemaversion>1.2</schemaversion>
  </metadata>
  <organizations default="org1">
    <organization identifier="org1">
      <title>{title}</title>
      <item identifier="item1" identifierref="res1" isvisible="true">
        <title>{title}</title>
      </item>
    </organization>
  </organizations>
  <resources>
    <resource identifier="res1" type="webcontent" adlcp:scormtype="sco" href="index.html">
{file_list}    </resource>
  </resources>
</manifest>
"#
        ),
        _ => format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest identifier="{id}" version="1" xmlns="http://www.imsglobal.org/xsd/imscp_v1p1" xmlns:adlcp="http://www.adlnet.org/xsd/adlcp_v1p3" xmlns:adlseq="http://www.adlnet.org/xsd/adlseq_v1p3" xmlns:adlnav="http://www.adlnet.org/xsd/adlnav_v1p3" xmlns:imsss="http://www.imsglobal.org/xsd/imsss" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://www.imsglobal.org/xsd/imscp_v1p1 imscp_v1p1.xsd http://www.adlnet.org/xsd/adlcp_v1p3 adlcp_v1p3.xsd http://www.adlnet.org/xsd/adlseq_v1p3 adlseq_v1p3.xsd http://www.adlnet.org/xsd/adlnav_v1p3 adlnav_v1p3.xsd http://www.imsglobal.org/xsd/imsss imsss_v1p0.xsd">
  <metadata>
    <schema>ADL SCORM</schema>
    <schemaversion>2004 4th Edition</schemaversion>
  </metadata>
  <organizations default="org1">
    <organization identifier="org1">
      <title>{title}</title>
      <item identifier="item1" identifierref="res1">
        <title>{title}</title>
      </item>
    </organization>
  </organizations>
  <resources>
    <resource identifier="res1" type="webcontent" adlcp:scormType="sco" href="index.html">
{file_list}    </resource>
  </resources>
</manifest>
"#
        ),
    }
}

/// Fill the shared fragments (LMS layer, question styles) into a player script.
fn shared(src: &str) -> String {
    src.replace("/*@LMS@*/", LMS_JS).replace("/*@QUESTIONS@*/", QUESTIONS_JS)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Project {
        let mut p = Project::new("Course & Co");
        p.slides.push(brainbend_doc::Slide {
            id: "s".into(),
            name: "s".into(),
            width: 10.0,
            height: 10.0,
            layers: vec![],
            triggers: vec![],
            kind: brainbend_doc::SlideKind::Normal,
            background: None,
            layout: None,
            fade_ms: 0, hide_backdrop: false,
            auto_advance: false, nav: Default::default(),
            notes: String::new(),
            scene: String::new(),
        });
        p
    }

    #[test]
    fn the_fonts_a_lesson_uses_travel_with_it_and_the_rest_are_named_as_fallbacks() {
        let mut p = tiny();
        p.player.font = "Lora".into();
        p.theme.heading_font = "No Such Font".into();
        let files = build(&p, Lms::None).unwrap();
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.iter().any(|n| n.starts_with("fonts/Lora-") && n.ends_with(".ttf")), "{names:?}");
        assert!(names.contains(&"fonts/Lora-license.txt"));
        assert!(names.iter().any(|n| n.starts_with("fonts/Open-Sans-")), "the player's own font always ships");
        assert!(!names.iter().any(|n| n.contains("No-Such")));
        let html = String::from_utf8(files.iter().find(|(n, _)| n == "index.html").unwrap().1.clone()).unwrap();
        assert!(html.contains("font-family:'Lora'") && html.contains("font-family:'Open Sans'"));
        let r = fonts_report(&p);
        assert!(r.embedded.contains(&"Lora".to_string()));
        assert_eq!(r.fallback, vec!["No Such Font".to_string()]);
    }

    #[test]
    fn scorm_packages_have_manifest_first_and_escaped_title() {
        for (v, ver) in [(Lms::Scorm12, "1.2"), (Lms::Scorm2004, "2004 4th Edition")] {
            let path = std::env::temp_dir().join(format!("brainbend-scorm-{}-{}.zip", std::process::id(), ver.len()));
            export_scorm(&tiny(), v, &path).unwrap();
            let mut z = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
            assert_eq!(z.by_index(0).unwrap().name().unwrap(), "imsmanifest.xml");
            let mut m = String::new();
            std::io::Read::read_to_string(&mut z.by_name("imsmanifest.xml").unwrap(), &mut m).unwrap();
            assert!(m.contains(ver), "{m}");
            assert!(m.contains("Course &amp; Co"));
            assert!(m.contains("href=\"index.html\""));
            assert!(z.by_name("index.html").is_ok());
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn media_assets_get_proper_extensions_and_are_referenced() {
        use brainbend_doc::{Asset, Kind, Object};
        let mut p = tiny();
        p.assets.insert("v".into(), Asset { name: "clip.mp4".into(), mime: "video/mp4".into(), bytes: vec![1, 2, 3], file: None });
        p.assets.insert("c".into(), Asset { name: "clip.vtt".into(), mime: "text/vtt".into(), bytes: b"WEBVTT\n".to_vec(), file: None });
        let mut o = Object::new("m", "clip", Kind::Video, 0.0, 0.0, 100.0, 60.0);
        o.asset = Some("v".into());
        o.captions = Some("c".into());
        p.slides[0].layers.push(brainbend_doc::Layer { id: "l".into(), name: "base".into(), objects: vec![o], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] });
        let files = build(&p, Lms::None).unwrap();
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.iter().any(|n| n.ends_with(".mp4")), "{names:?}");
        assert!(names.iter().any(|n| n.ends_with(".vtt")), "{names:?}");
        let html = String::from_utf8(files.iter().find(|(n, _)| n == "index.html").unwrap().1.clone()).unwrap();
        assert!(html.contains("\"kind\":\"Video\""));
        assert!(html.contains("assets/a1.mp4") || html.contains("assets/a0.mp4"));
    }

    #[test]
    fn master_shapes_backgrounds_and_background_pictures_are_published() {
        use brainbend_doc::{Asset, Background, Kind, Object, Rgba};
        let mut p = tiny();
        p.sanitize();
        p.assets.insert("bg".into(), Asset { name: "bg.png".into(), mime: "image/png".into(), bytes: vec![1, 2, 3], file: None });
        p.master.background = Some(Background::Image { asset: "bg".into() });
        p.master.objects.push(Object::new("logo", "Logo", Kind::Rect, 0.0, 0.0, 5.0, 5.0));
        p.slides[0].layout = Some("layout-blank".into());
        let files = build(&p, Lms::None).unwrap();
        assert!(files.iter().any(|(n, _)| n.starts_with("assets/") && n.ends_with(".png")), "background picture ships");
        let html = String::from_utf8(files.iter().find(|(n, _)| n == "index.html").unwrap().1.clone()).unwrap();
        assert!(html.contains("\"asset\":\"bg\""), "slide carries the resolved background");
        assert!(html.contains("bd-"), "master shape is baked into the slide");
        assert!(!html.contains("layout-blank"), "layouts are not shipped");
        let print = print_html(&p);
        assert!(print.contains("data:image/png;base64,"), "handout inlines the background picture");
        p.master.background = Some(Background::gradient(90.0, Rgba(0, 0, 0, 255), Rgba(255, 255, 255, 255)));
        assert!(print_html(&p).contains("linear-gradient(90deg"));
    }

    #[test]
    fn a_360_picture_ships_as_a_script_so_a_page_opened_from_disk_can_use_it() {
        use brainbend_doc::{pano::Pano, Asset, Kind, Object, Slide};
        let mut p = Project::new("t");
        p.assets.insert("pic".into(), Asset { name: "a.jpg".into(), mime: "image/jpeg".into(), bytes: b"abcd".to_vec(), file: None });
        let mut s = Slide::new("s", "S", 1920.0, 1080.0);
        let mut o = Object::new("o", "Tour", Kind::Pano(Pano::new()), 0.0, 0.0, 800.0, 450.0);
        o.asset = Some("pic".into());
        s.layers[0].objects = vec![o];
        p.slides = vec![s];
        let files = build(&p, Lms::None).unwrap();
        let js = files.iter().find(|(n, _)| n == "assets/pano0.js").expect("script");
        assert_eq!(String::from_utf8(js.1.clone()).unwrap(), "(window.__panoPix=window.__panoPix||{})[\"pic\"]=\"data:image/jpeg;base64,YWJjZA==\";");
        assert!(!files.iter().any(|(n, _)| n.ends_with(".jpg")), "no second copy as a picture file");
        let html = String::from_utf8(files.iter().find(|(n, _)| n == "index.html").unwrap().1.clone()).unwrap();
        assert!(html.find("assets/pano0.js").unwrap() < html.find("window.__BRAINBEND__").unwrap());
    }

    #[test]
    fn unreferenced_assets_are_not_exported() {
        use brainbend_doc::Asset;
        let mut p = tiny();
        p.assets.insert("orphan".into(), Asset { name: "old.png".into(), mime: "image/png".into(), bytes: vec![9; 100], file: None });
        let files = build(&p, Lms::None).unwrap();
        assert!(!files.iter().any(|(n, _)| n.starts_with("assets/")), "orphan image must not ship");
    }

    #[test]
    fn plain_export_has_no_manifest_and_lms_mode_is_set() {
        let files = build(&tiny(), Lms::None).unwrap();
        assert!(!files.iter().any(|(p, _)| p == "imsmanifest.xml"));
        let html = String::from_utf8(files.iter().find(|(p, _)| p == "index.html").unwrap().1.clone()).unwrap();
        assert!(html.contains("\"lms\":\"none\""));
    }

    #[test]
    fn title_is_escaped_and_data_cannot_close_script() {
        let mut p = Project::new("<b>x</b>");
        p.slides.push(brainbend_doc::Slide {
            id: "s".into(),
            name: "</script>".into(),
            width: 10.0,
            height: 10.0,
            layers: vec![],
            triggers: vec![],
            kind: brainbend_doc::SlideKind::Normal,
            background: None,
            layout: None,
            fade_ms: 0, hide_backdrop: false,
            auto_advance: false, nav: Default::default(),
            notes: String::new(),
            scene: String::new(),
        });
        let dir = std::env::temp_dir().join(format!("brainbend-test-{}", std::process::id()));
        export(&p, &dir).unwrap();
        let html = std::fs::read_to_string(dir.join("index.html")).unwrap();
        assert!(html.contains("&lt;b&gt;x&lt;/b&gt;"));
        assert_eq!(html.matches("</script>").count(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// `:root` custom properties for the player's colours, font and text size.
fn player_vars(p: &brainbend_doc::PlayerSettings) -> String {
    // The two bundled faces come first in the stack so a font that is not installed falls back to them.
    let font = format!("\"{}\",\"Open Sans\",Montserrat,Arial,sans-serif", p.font.replace(['"', '\\', '<', '>', ';', '{', '}'], ""));
    format!("{};--p-font:{};--p-scale:{}", p.palette().css_vars(), font, p.font_scale.clamp(70, 160) as f32 / 100.0)
}
