//! Export a project, check every part is well-formed XML, and read it back with the importer.
use std::io::{Cursor, Read};
use brainbend_doc::*;

fn png() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 30, 30, 255]));
    let mut b = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img).write_to(&mut b, image::ImageFormat::Png).unwrap();
    b.into_inner()
}

fn project() -> Project {
    let mut p = Project::new("Demo");
    p.theme = Theme::presets().remove(1);
    p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: png(), file: None });
    p.assets.insert("clip".into(), Asset { name: "clip.mp4".into(), mime: "video/mp4".into(), bytes: vec![0, 0, 0, 24, 102, 116, 121, 112], file: None });
    for (i, name) in ["Intro", "Details", "Quiz"].iter().enumerate() {
        p.slides.push(Slide::new(&format!("s{}", i + 1), name, 1920.0, 1080.0));
    }
    p.sanitize();
    p.scenes = vec![Scene { id: "a".into(), name: "Part A".into(), bank: None }, Scene { id: "b".into(), name: "Part B".into(), bank: None }];
    p.slides[0].scene = "a".into();
    p.slides[1].scene = "b".into();
    p.slides[2].scene = "b".into();
    p.master.background = Some(Background::Solid(Rgba(250, 250, 245, 255)));
    p.master.objects.push({ let mut o = Object::new("logo", "Logo", Kind::Rect, 40.0, 1000.0, 300.0, 40.0); o.fill = Some(Rgba(3, 83, 158, 255)); o });
    p.layouts[1].background = Some(Background::gradient(90.0, Rgba(255, 255, 255, 255), Rgba(200, 220, 255, 255)));

    // Slide 1 follows "Title and Content" and fills its placeholders.
    let mut n = 0;
    p.apply_layout(0, "layout-content", &mut || { n += 1; format!("ph{n}") });
    let objs = &mut p.slides[0].layers[0].objects;
    objs[0].text = "Welcome".into();
    objs[1].text = "First point\nSecond point".into();
    let mut rich = Object::new("rich", "Rich", Kind::Text, 100.0, 700.0, 900.0, 120.0);
    rich.text = "Plain bold".into();
    rich.rich = Some(vec![Paragraph { runs: vec![Run { text: "Plain ".into(), ..Default::default() }, Run { text: "bold".into(), bold: Some(true), color: Some(Rgba(200, 0, 0, 255)), ..Default::default() }], ..Default::default() }]);
    rich.style.size = 40.0;
    rich.line_spacing = 1.5;
    objs.push(rich);
    let mut rect = Object::new("rect", "Card", Kind::Rect, 1000.0, 700.0, 400.0, 200.0);
    rect.fill = Some(Rgba(240, 120, 30, 255));
    rect.radius = 24.0;
    rect.stroke = Some(Stroke { color: Rgba(0, 0, 0, 255), width: 4.0 });
    rect.shadow = Some(Shadow { dx: 6.0, dy: 6.0, blur: 10.0, spread: 0.0, color: Rgba(0, 0, 0, 90) });
    rect.glow = Some(Glow { radius: 12.0, color: Rgba(255, 200, 0, 150) });
    rect.reflection = Some(Reflection { size: 0.5, opacity: 0.4, gap: 4.0 });
    rect.soft_edge = 6.0;
    rect.rotation = 15.0;
    rect.text = "Go to details".into();
    rect.style.align = Align::Center;
    rect.alt = "Jump to the details slide".into();
    objs.push(rect);
    let mut grad = Object::new("grad", "Gradient box", Kind::Rect, 100.0, 880.0, 400.0, 100.0);
    grad.fill = Some(Rgba(100, 100, 255, 255));
    grad.gradient = Some(Gradient { angle: 180.0, stops: vec![(0.0, Rgba(255, 0, 0, 255)), (1.0, Rgba(0, 0, 255, 255))] });
    objs.push(grad);
    objs.push({
        let mut o = Object::new("circle", "Circle", Kind::Ellipse, 1500.0, 700.0, 200.0, 200.0);
        o.fill = Some(Rgba(0, 160, 120, 255));
        o.entrance = Some(Entrance { kind: EffectKind::SlideLeft, delay_ms: 200, duration_ms: 600 });
        o.emphasis = Some(Emphasis { kind: EmphasisKind::Spin, delay_ms: 100, duration_ms: 1000, repeat: 2 });
        o.motion = Some(MotionPath { delay_ms: 300, duration_ms: 1500, ..MotionPath::preset(PathPreset::Arc) });
        o
    });
    objs.push({
        let mut o = Object::new("tri", "Triangle", Kind::Path(vec![PathCmd::Move(0.5, 0.0), PathCmd::Line(1.0, 1.0), PathCmd::Line(0.0, 1.0), PathCmd::Close]), 1500.0, 400.0, 200.0, 200.0);
        o.fill = Some(Rgba(90, 30, 160, 255));
        o.entrance = Some(Entrance { kind: EffectKind::Fade, delay_ms: 0, duration_ms: 500 });
        o.click_steps.entrance = 1;
        o.exit = Some(Entrance { kind: EffectKind::Wipe, delay_ms: 100, duration_ms: 300 });
        o.click_steps.exit = 2;
        o
    });
    objs.push({ let mut o = Object::new("line", "Rule", Kind::LineUp, 100.0, 650.0, 600.0, 40.0); o.stroke = Some(Stroke { color: Rgba(30, 30, 30, 255), width: 6.0 }); o });
    objs.push({ let mut o = Object::new("img", "Picture", Kind::Image, 1000.0, 100.0, 300.0, 200.0); o.asset = Some("pic".into()); o.alt = "A red square".into(); o });
    objs.push({ let mut o = Object::new("vid", "Clip", Kind::Video, 1400.0, 100.0, 400.0, 225.0); o.asset = Some("clip".into()); o });
    p.slides[0].notes = "Open warmly.\nMention the agenda.".into();
    p.slides[0].fade_ms = 700;
    p.slides[0].triggers.push(Trigger { id: "t1".into(), owner: Owner::Object("rect".into()), event: Event::Click, action: Action::JumpToSlide("s2".into()), conditions: vec![] });

    // Slide 2 has no layout and its own picture background.
    p.slides[1].layout = None;
    p.slides[1].background = Some(Background::Image { asset: "pic".into() });
    p.slides[1].layers[0].objects.push({ let mut o = Object::new("t2", "Text 2", Kind::Text, 100.0, 100.0, 800.0, 100.0); o.text = "Details & <more>".into(); o });
    // Slide 3 is a question.
    p.slides[2].kind = SlideKind::Question(Question::new_single());
    p
}

fn parts(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..z.len()).map(|i| {
        let mut f = z.by_index(i).unwrap();
        let mut b = vec![];
        f.read_to_end(&mut b).unwrap();
        (f.name().unwrap().to_string(), b)
    }).collect()
}

#[test]
fn every_part_is_well_formed_and_every_relationship_resolves() {
    let (bytes, _) = brainbend_export_pptx::export(&project()).unwrap();
    let all = parts(&bytes);
    let names: Vec<&str> = all.iter().map(|(n, _)| n.as_str()).collect();
    for (name, body) in &all {
        if name.ends_with(".xml") || name.ends_with(".rels") {
            roxmltree::Document::parse(std::str::from_utf8(body).unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        if name.ends_with(".rels") {
            let doc = roxmltree::Document::parse(std::str::from_utf8(body).unwrap()).unwrap();
            let dir = name.replace("_rels/", "");
            let base = dir.rsplit_once('/').map(|x| x.0).unwrap_or("");
            let base = base.trim_end_matches(".rels");
            let owner_dir = if name == "_rels/.rels" { "" } else { name.rsplit_once("/_rels/").map(|x| x.0).unwrap_or("") };
            let _ = base;
            for r in doc.descendants().filter(|n| n.tag_name().name() == "Relationship") {
                if r.attribute("TargetMode") == Some("External") { continue; }
                let mut segs: Vec<&str> = if owner_dir.is_empty() { vec![] } else { owner_dir.split('/').collect() };
                for s in r.attribute("Target").unwrap().split('/') { match s { ".." => { segs.pop(); } "." | "" => {} s => segs.push(s) } }
                let target = segs.join("/");
                assert!(names.contains(&target.as_str()), "{name}: {} points at missing {target}", r.attribute("Id").unwrap());
            }
        }
    }
    assert!(names.contains(&"[Content_Types].xml"));
    let ct = String::from_utf8(all.iter().find(|(n, _)| n == "[Content_Types].xml").unwrap().1.clone()).unwrap();
    for (n, _) in &all {
        if n.ends_with(".xml") && n != "[Content_Types].xml" { assert!(ct.contains(&format!("/{n}")), "{n} has no content type override"); }
    }
}

#[test]
fn a_project_survives_export_and_import() {
    let src = project();
    let (bytes, notes) = brainbend_export_pptx::export(&src).unwrap();
    assert!(notes.iter().any(|n| n.contains("Question slides")));
    let back = brainbend_import_pptx::import(Cursor::new(bytes), "back").unwrap();
    assert_eq!(back.slides.len(), 3);
    assert_eq!(back.slides[0].width, 1920.0);
    assert_eq!(back.scenes.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Part A", "Part B"]);
    assert_eq!(back.theme.heading_font, src.theme.heading_font);
    assert_eq!(back.theme.colors[4], src.theme.colors[4]);
    assert_eq!(back.master.background, Some(Background::Solid(Rgba(250, 250, 245, 255))));
    assert!(back.master.objects.iter().any(|o| o.name == "Logo"));
    assert!(back.layouts.iter().any(|l| l.name == "Title and Content" && matches!(l.background, Some(Background::Gradient { .. }))));

    let s = &back.slides[0];
    assert!(s.layout.is_some());
    let find = |n: &str| s.layers[0].objects.iter().find(|o| o.name == n).unwrap_or_else(|| panic!("missing {n}: {:?}", s.layers[0].objects.iter().map(|o| &o.name).collect::<Vec<_>>()));
    let title = s.layers[0].objects.iter().find(|o| o.placeholder == Some(PlaceholderKind::Title)).unwrap();
    assert_eq!(title.text, "Welcome");
    assert!(s.layers[0].objects.iter().any(|o| o.placeholder == Some(PlaceholderKind::Body) && o.text.contains("Second point")));
    let rich = find("Rich");
    assert_eq!(rich.text, "Plain bold");
    assert!(rich.rich.as_ref().unwrap()[0].runs.iter().any(|r| r.text == "bold" && r.bold == Some(true) && r.color == Some(Rgba(200, 0, 0, 255))));
    assert!((rich.line_spacing - 1.5).abs() < 0.02, "{}", rich.line_spacing);
    let card = find("Card");
    assert_eq!(card.fill, Some(Rgba(240, 120, 30, 255)));
    assert!((card.radius - 24.0).abs() < 1.0);
    assert!((card.rotation - 15.0).abs() < 0.1);
    assert_eq!(card.stroke.unwrap().color, Rgba(0, 0, 0, 255));
    assert!((card.stroke.unwrap().width - 4.0).abs() < 0.6);
    assert!(card.shadow.is_some());
    let glow = card.glow.expect("glow survives");
    assert!((glow.radius - 12.0).abs() < 1.0 && glow.color.0 == 255 && glow.color.1 == 200, "{glow:?}");
    let refl = card.reflection.expect("reflection survives");
    assert!((refl.size - 0.5).abs() < 0.01 && (refl.opacity - 0.4).abs() < 0.01 && (refl.gap - 4.0).abs() < 1.0, "{refl:?}");
    assert!((card.soft_edge - 6.0).abs() < 1.0, "soft edges survive: {}", card.soft_edge);
    assert_eq!(card.alt, "Jump to the details slide");
    assert!(matches!(&s.triggers[0].action, Action::JumpToSlide(id) if *id == back.slides[1].id), "link survives");
    assert_eq!(find("Gradient box").gradient.as_ref().unwrap().stops.len(), 2);
    assert!(matches!(find("Circle").kind, Kind::Ellipse));
    assert!(matches!(find("Triangle").kind, Kind::Path(_)));
    assert!(matches!(find("Rule").kind, Kind::LineUp));
    assert!(matches!(find("Picture").kind, Kind::Image));
    assert!(matches!(find("Clip").kind, Kind::Video));
    // Click steps come back as click steps.
    let tri = find("Triangle");
    assert_eq!((tri.click_steps.entrance, tri.click_steps.exit), (1, 2));
    assert_eq!(tri.exit.map(|x| (x.kind, x.delay_ms, x.duration_ms)), Some((EffectKind::Wipe, 100, 300)));
    assert!(tri.timing.is_none(), "a click exit is not a timeline end");
    // Animations travel as one automatic sequence and come back as the same effects.
    let circle = find("Circle");
    let e = circle.entrance.expect("entrance");
    assert_eq!((e.kind, e.delay_ms, e.duration_ms), (EffectKind::SlideLeft, 200, 600));
    let em = circle.emphasis.expect("emphasis");
    assert_eq!((em.kind, em.delay_ms, em.duration_ms, em.repeat), (EmphasisKind::Spin, 100, 1000, 2));
    let m = circle.motion.as_ref().expect("motion path");
    let want = MotionPath::preset(PathPreset::Arc);
    assert!(m.smooth && m.points.len() == want.points.len() && (m.delay_ms, m.duration_ms) == (300, 1500));
    assert!(m.points.iter().zip(&want.points).all(|(a, b)| (a.0 - b.0).abs() < 1.5 && (a.1 - b.1).abs() < 1.5), "{:?}", m.points);
    assert_eq!(s.notes, "Open warmly.\nMention the agenda.");
    assert_eq!(s.fade_ms, 700);

    assert!(matches!(back.slides[1].background, Some(Background::Image { .. })));
    assert!(back.slides[1].layers[0].objects.iter().any(|o| o.text == "Details & <more>"), "text is escaped and restored");
    assert!(back.slides[2].layers[0].objects.iter().any(|o| o.text.contains("Choice A")), "question exported as text");
}

/// `BRAINBEND_PPTX_OUT=/path/demo.pptx cargo test -p brainbend-export-pptx writes_a_sample` leaves a file to open in PowerPoint.
#[test]
fn tables_export_as_native_tables_and_come_back_editable() {
    let mut p = project();
    let mut n = 0;
    let mut cells = brainbend_doc::tables::new_table("tbl", 3, 3, (200.0, 300.0, 900.0, 300.0), &p.theme, &mut || { n += 1; format!("c{n}") });
    cells[0].text = "Header & <one>".into();
    cells[4].text = "middle".into();
    brainbend_doc::tables::merge(&mut cells, "tbl", (1, 1), (2, 2));
    let red = Some(Stroke { color: Rgba(200, 0, 0, 255), width: 6.0 });
    brainbend_doc::tables::set_borders(&mut cells, "tbl", (0, 0, 0, 2), brainbend_doc::tables::BorderSet::Bottom, red);
    let count = cells.len();
    p.slides[1].layers[0].objects.extend(cells);
    let (bytes, _) = brainbend_export_pptx::export(&p).unwrap();
    let xml = String::from_utf8(parts(&bytes).into_iter().find(|(n, _)| n == "ppt/slides/slide2.xml").unwrap().1).unwrap();
    assert!(xml.contains("<a:tbl>") && xml.contains("gridSpan=\"2\"") && xml.contains("rowSpan=\"2\"") && xml.contains("hMerge=\"1\"") && xml.contains("vMerge=\"1\""));
    assert!(!xml.contains("Group "), "a table is not a plain group");
    assert_eq!(xml.matches("<a:gridCol ").count(), 3);
    assert_eq!(xml.matches("<a:tr ").count(), 3);
    assert!(xml.contains("Header &amp; &lt;one&gt;"));
    let back = brainbend_import_pptx::import(Cursor::new(bytes), "back").unwrap();
    let objs = &back.slides[1].layers[0].objects;
    let tbl: Vec<&Object> = objs.iter().filter(|o| o.table.is_some()).collect();
    assert_eq!(tbl.len(), count, "same number of cells back");
    let head = tbl.iter().find(|o| o.table == Some((0, 0))).unwrap();
    assert_eq!(head.text, "Header & <one>");
    assert_eq!(head.fill, Some(p.theme.color(4)));
    let sd = brainbend_doc::tables::sides(head);
    assert_eq!(sd[3].map(|s| s.color), Some(Rgba(200, 0, 0, 255)), "per-side border survives");
    assert!((sd[3].unwrap().width - 6.0).abs() < 0.6);
    assert_eq!(sd[0].map(|s| s.color), Some(Rgba(255, 255, 255, 255)), "the other sides keep the style's line");
    let big = tbl.iter().find(|o| o.table == Some((1, 1))).unwrap();
    assert_eq!(big.table_span, Some((2, 2)));
    assert!((big.w - 600.0).abs() < 1.5 && (big.h - 200.0).abs() < 1.5, "{} x {}", big.w, big.h);
    assert_eq!(brainbend_doc::tables::dims(objs, head.group.as_deref().unwrap()), (3, 3));
}

#[test]
fn writes_a_sample() {
    let Some(out) = std::env::var_os("BRAINBEND_PPTX_OUT") else { return };
    std::fs::write(out, brainbend_export_pptx::export(&project()).unwrap().0).unwrap();
}

/// `BRAINBEND_PPTX_FILES=a.pptx:b.pptx` imports real decks, exports them and imports the result again.
#[test]
fn real_decks_round_trip() {
    let Ok(files) = std::env::var("BRAINBEND_PPTX_FILES") else { return };
    for f in files.split(':').filter(|f| !f.is_empty()) {
        let a = brainbend_import_pptx::import(std::fs::File::open(f).unwrap(), "a").unwrap();
        let (bytes, notes) = brainbend_export_pptx::export(&a).unwrap();
        for (name, body) in parts(&bytes) {
            if name.ends_with(".xml") || name.ends_with(".rels") { roxmltree::Document::parse(std::str::from_utf8(&body).unwrap()).unwrap_or_else(|e| panic!("{f}: {name}: {e}")); }
        }
        let b = brainbend_import_pptx::import(Cursor::new(bytes), "b").unwrap();
        let count = |p: &Project| p.slides.iter().map(|s| s.layers[0].objects.len()).collect::<Vec<_>>();
        println!("{f}: slides {} -> {}, objects {:?} -> {:?}, layouts {} -> {}, notes {:?}", a.slides.len(), b.slides.len(), count(&a), count(&b), a.layouts.len(), b.layouts.len(), notes);
        assert_eq!(a.slides.len(), b.slides.len());
        assert_eq!(count(&a), count(&b), "{f}");
    }
}

/// `BRAINBEND_PPTX_DIR=/dir cargo test -p brainbend-export-pptx writes_bisect_samples` writes small decks
/// (empty, text only, table only, merged table) to open in PowerPoint and find what it objects to.
#[test]
fn writes_bisect_samples() {
    let Some(dir) = std::env::var_os("BRAINBEND_PPTX_DIR") else { return };
    let dir = std::path::PathBuf::from(dir);
    let one = |name: &str, f: &dyn Fn(&mut Project)| {
        let mut p = Project::new("Sample");
        p.slides.push(Slide::new("s1", "One", 1920.0, 1080.0));
        p.sanitize();
        f(&mut p);
        std::fs::write(dir.join(name), brainbend_export_pptx::export(&p).unwrap().0).unwrap();
    };
    let table = |p: &mut Project, merge: bool| {
        let mut n = 0;
        let mut cells = brainbend_doc::tables::new_table("tbl", 3, 3, (200.0, 300.0, 900.0, 300.0), &p.theme, &mut || { n += 1; format!("c{n}") });
        cells[0].text = "Header".into();
        if merge { brainbend_doc::tables::merge(&mut cells, "tbl", (1, 1), (2, 2)); }
        p.slides[0].layers[0].objects.extend(cells);
    };
    one("1-empty.pptx", &|_| {});
    one("2-text.pptx", &|p| p.slides[0].layers[0].objects.push({ let mut o = Object::new("t", "Text", Kind::Text, 100.0, 100.0, 800.0, 100.0); o.text = "Hello".into(); o }));
    one("3-table.pptx", &|p| table(p, false));
    one("4-table-merged.pptx", &|p| table(p, true));
    // Animations, a few kinds at a time, so a problem can be traced to one group.
    let boxes = |p: &mut Project, n: usize| -> Vec<usize> {
        let objs = &mut p.slides[0].layers[0].objects;
        let first = objs.len();
        for i in 0..n {
            let mut o = Object::new(&format!("b{i}"), &format!("Box {i}"), Kind::Rect, 100.0 + (i % 4) as f32 * 420.0, 150.0 + (i / 4) as f32 * 300.0, 360.0, 200.0);
            o.fill = Some(Rgba(3, 83, 158, 255));
            o.text = format!("Box {i}");
            o.style.color = Rgba(255, 255, 255, 255);
            objs.push(o);
        }
        (first..first + n).collect()
    };
    // A layer popup: click "Open" on the first slide to see the popup slide, "Close" to come back.
    one("10-layers.pptx", &|p| {
        p.slides.push(Slide::new("s2", "Two", 1920.0, 1080.0));
        let mut open = Object::new("open", "Open", Kind::Rect, 100.0, 100.0, 400.0, 120.0);
        open.fill = Some(Rgba(3, 83, 158, 255));
        open.text = "Open popup".into();
        open.style.color = Rgba(255, 255, 255, 255);
        let mut close = Object::new("close", "Close", Kind::Rect, 700.0, 500.0, 400.0, 120.0);
        close.fill = Some(Rgba(160, 30, 30, 255));
        close.text = "Close popup".into();
        close.style.color = Rgba(255, 255, 255, 255);
        p.slides[0].layers[0].objects.push(open);
        p.slides[0].layers.push(Layer { id: "pop".into(), name: "Popup".into(), objects: vec![close], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] });
        let t = |id: &str, owner: &str, action: Action| Trigger { id: id.into(), owner: Owner::Object(owner.into()), event: Event::Click, action, conditions: vec![] };
        p.slides[0].triggers.push(t("t1", "open", Action::ShowLayer("pop".into())));
        p.slides[0].triggers.push(t("t2", "close", Action::HideLayer("pop".into())));
    });
    one("5-anim-entrances.pptx", &|p| {
        let idx = boxes(p, 4);
        for (k, kind) in [EffectKind::Fade, EffectKind::Wipe, EffectKind::Zoom, EffectKind::SlideLeft].into_iter().enumerate() {
            p.slides[0].layers[0].objects[idx[k]].entrance = Some(Entrance { kind, delay_ms: k as u32 * 300, duration_ms: 700 });
        }
    });
    one("6-anim-emphasis.pptx", &|p| {
        let idx = boxes(p, 7);
        for (k, kind) in EmphasisKind::ALL.into_iter().enumerate() {
            p.slides[0].layers[0].objects[idx[k]].emphasis = Some(Emphasis { kind, delay_ms: 500, duration_ms: 1000, repeat: 2 });
        }
    });
    one("7-anim-motion.pptx", &|p| {
        let idx = boxes(p, 2);
        p.slides[0].layers[0].objects[idx[0]].motion = Some(MotionPath::preset(PathPreset::Right));
        p.slides[0].layers[0].objects[idx[1]].motion = Some(MotionPath::preset(PathPreset::Circle));
    });
    one("9-anim-clicks.pptx", &|p| {
        let idx = boxes(p, 4);
        let o = &mut p.slides[0].layers[0].objects;
        for k in [0, 1] { o[idx[k]].entrance = Some(Entrance { kind: EffectKind::Fade, delay_ms: 0, duration_ms: 500 }); o[idx[k]].click_steps.entrance = 1; }
        o[idx[1]].emphasis = Some(Emphasis { kind: EmphasisKind::Spin, delay_ms: 0, duration_ms: 800, repeat: 1 });
        o[idx[1]].click_steps.emphasis = 2;
        o[idx[2]].exit = Some(Entrance { kind: EffectKind::Fade, delay_ms: 0, duration_ms: 500 });
        o[idx[2]].click_steps.exit = 3;
        o[idx[3]].entrance = Some(Entrance { kind: EffectKind::SlideLeft, delay_ms: 0, duration_ms: 500 });
    });
    one("8-anim-timed-and-exit.pptx", &|p| {
        let idx = boxes(p, 3);
        p.slides[0].layers[0].duration_ms = Some(6000);
        p.slides[0].layers[0].objects[idx[0]].timing = Some(Timing { start_ms: 1000, dur_ms: Some(3000) });
        p.slides[0].layers[0].objects[idx[1]].exit = Some(Entrance { kind: EffectKind::SlideRight, delay_ms: 0, duration_ms: 600 });
        p.slides[0].layers[0].objects[idx[2]].timing = Some(Timing { start_ms: 500, dur_ms: None });
        p.slides[0].layers[0].objects[idx[2]].entrance = Some(Entrance { kind: EffectKind::Fade, delay_ms: 0, duration_ms: 500 });
    });
}

#[test]
fn extra_layers_become_hidden_slides_with_jump_links() {
    let mut p = Project::new("Layers");
    p.slides.push(Slide::new("s1", "Home", 1920.0, 1080.0));
    p.slides.push(Slide::new("s2", "Next", 1920.0, 1080.0));
    p.sanitize();
    let open = Object::new("open", "Open", Kind::Rect, 100.0, 100.0, 300.0, 100.0);
    let close = Object::new("close", "Close", Kind::Rect, 200.0, 200.0, 300.0, 100.0);
    p.slides[0].layers[0].objects.push(open);
    p.slides[0].layers.push(Layer { id: "pop".into(), name: "Popup".into(), objects: vec![close], duration_ms: None, modal: true, hide_others: false, pause_base: false, group_names: Default::default(), focus_order: vec![] });
    let t = |id: &str, owner: &str, action: Action| Trigger { id: id.into(), owner: Owner::Object(owner.into()), event: Event::Click, action, conditions: vec![] };
    p.slides[0].triggers.push(t("t1", "open", Action::ShowLayer("pop".into())));
    p.slides[0].triggers.push(t("t2", "close", Action::HideLayer("pop".into())));
    let (bytes, notes) = brainbend_export_pptx::export(&p).unwrap();
    assert!(notes.iter().any(|n| n.contains("hidden slides")), "{notes:?}");
    let files = parts(&bytes);
    let get = |n: &str| String::from_utf8(files.iter().find(|f| f.0 == n).unwrap_or_else(|| panic!("missing {n}")).1.clone()).unwrap();
    // Home, Home - Popup (hidden), Next
    assert!(files.iter().any(|f| f.0 == "ppt/slides/slide3.xml"));
    assert!(get("ppt/slides/slide2.xml").contains("show=\"0\""));
    assert!(!get("ppt/slides/slide1.xml").contains("show=\"0\""));
    assert!(get("ppt/slides/slide1.xml").contains("hlinksldjump"));
    assert!(get("ppt/slides/_rels/slide1.xml.rels").contains("slide2.xml"));
    assert!(get("ppt/slides/slide2.xml").contains("hlinksldjump"));
    assert!(get("ppt/slides/_rels/slide2.xml.rels").contains("slide1.xml"));
    // The popup slide carries the base objects too.
    let s2 = get("ppt/slides/slide2.xml");
    assert!(s2.contains("Open") && s2.contains("Close"));
    // And it still imports (the importer leaves hidden slides out, as it does for any deck).
    let back = brainbend_import_pptx::import(Cursor::new(bytes.clone()), "back").unwrap();
    assert_eq!(back.slides.len(), 2);
}

#[test]
fn text_links_survive_export_and_import() {
    let mut p = Project::new("Links");
    p.slides.push(Slide::new("s1", "One", 1920.0, 1080.0));
    p.sanitize();
    let mut o = Object::new("t", "Text", Kind::Text, 100.0, 100.0, 800.0, 100.0);
    o.text = "See docs now".into();
    o.rich = Some(vec![Paragraph { runs: vec![
        Run { text: "See ".into(), ..Default::default() },
        Run { text: "docs".into(), link: Some("https://example.com/docs".into()), ..Default::default() },
        Run { text: " now".into(), ..Default::default() },
    ], ..Default::default() }]);
    p.slides[0].layers[0].objects.push(o);
    let (bytes, _) = brainbend_export_pptx::export(&p).unwrap();
    let back = brainbend_import_pptx::import(Cursor::new(bytes), "back").unwrap();
    let t = back.slides[0].layers[0].objects.iter().find(|o| o.text.contains("docs")).unwrap();
    let linked: Vec<_> = t.rich.as_ref().unwrap().iter().flat_map(|p| p.runs.iter()).filter(|r| r.link.is_some()).collect();
    assert_eq!(linked.len(), 1);
    assert_eq!((linked[0].text.as_str(), linked[0].link.as_deref()), ("docs", Some("https://example.com/docs")));
    assert!(back.slides[0].triggers.is_empty(), "a link on some words does not make the whole box clickable");
}

#[test]
fn a_picture_crop_is_written_as_a_source_rectangle() {
    let mut p = Project::new("Crop");
    p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: png(), file: None });
    p.slides.push(Slide::new("s1", "One", 1920.0, 1080.0));
    p.sanitize();
    let mut o = Object::new("img", "Picture", Kind::Image, 100.0, 100.0, 400.0, 300.0);
    o.asset = Some("pic".into());
    o.crop = Some([0.1, 0.0, 0.25, 0.05]);
    p.slides[0].layers[0].objects.push(o);
    let (bytes, _) = brainbend_export_pptx::export(&p).unwrap();
    let slide = parts(&bytes).into_iter().find(|(n, _)| n == "ppt/slides/slide1.xml").map(|(_, b)| String::from_utf8(b).unwrap()).unwrap();
    assert!(slide.contains(r#"<a:srcRect l="10000" t="0" r="25000" b="5000"/>"#), "{slide}");
}

#[test]
fn a_picture_cropped_to_a_shape_keeps_it_through_powerpoint() {
    let mut p = Project::new("Shape");
    p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: png(), file: None });
    p.slides.push(Slide::new("s1", "One", 1920.0, 1080.0));
    p.sanitize();
    for (i, shape) in PictureShape::ALL.into_iter().enumerate() {
        let mut o = Object::new(&format!("img{i}"), "Picture", Kind::Image, 100.0 * i as f32, 100.0, 80.0, 80.0);
        o.asset = Some("pic".into());
        o.mask = Some(shape);
        p.slides[0].layers[0].objects.push(o);
    }
    let (bytes, _) = brainbend_export_pptx::export(&p).unwrap();
    let slide = parts(&bytes).into_iter().find(|(n, _)| n == "ppt/slides/slide1.xml").map(|(_, b)| String::from_utf8(b).unwrap()).unwrap();
    assert!(slide.contains(r#"<a:prstGeom prst="ellipse">"#) && slide.contains(r#"prst="roundRect""#) && slide.contains(r#"prst="star5""#), "{slide}");
    let back = brainbend_import_pptx::import(Cursor::new(bytes), "Shape").unwrap();
    let masks: Vec<_> = back.slides[0].layers[0].objects.iter().filter(|o| matches!(o.kind, Kind::Image)).map(|o| o.mask).collect();
    assert_eq!(masks, PictureShape::ALL.map(Some).to_vec());
}
