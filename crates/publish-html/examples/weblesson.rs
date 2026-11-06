// A Web Lesson for the player tests: weblesson <out_dir | out.zip> [scorm12|scorm2004]
use brainbend_doc::lesson::*;
use brainbend_doc::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap();
    let mode = args.next();
    let mut p = Project::new_web_lesson("Safety lesson", ProjectKind::WebLesson);
    p.pass_mark = 100;
    p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 252, 207, 192, 80, 15, 0, 4, 133, 1, 128, 132, 169, 140, 33, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130], file: None });
    let b = |id: &str, kind| Block { id: id.into(), kind };
    let item = |t: &str, body: &str| Item { title: t.into(), body: body.into() };
    let mut q = Question::new_single();
    q.prompt = "Which is safe?".into();
    q.choices[0].text = "Wear a helmet".into();
    // WL_THEME=<preset index> and WL_PRIMARY=<rrggbb> let the color audit try other looks.
    if let Some(i) = std::env::var("WL_THEME").ok().and_then(|v| v.parse::<usize>().ok()) {
        p.theme = Theme::presets().remove(i);
    }
    let l = p.lesson.as_mut().unwrap();
    if let Some(h) = std::env::var("WL_PRIMARY").ok().filter(|h| h.len() == 6) {
        let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap();
        l.primary = Some(Rgba(v(0), v(2), v(4), 255));
    }
    l.cover = Some(Cover { subtitle: "Stay safe at work".into(), description: "Learn the habits that keep you and your team safe on the job.".into(), image: None });
    l.sections = vec![
        Section { id: "s1".into(), title: "Basics".into(), summary: "Why safety matters and the core ideas.".into(), blocks: vec![
            b("h1", BlockKind::Heading { text: "Why it matters".into(), level: 2 }),
            b("t1", BlockKind::Text { text: "Read **this** and *that*.\n\n- first point\n- second point\n\n[Safe link](https://example.com/safe) and [bad link](javascript:alert(1)) <img src=x onerror=alert(2)>".into() }),
            b("i1", BlockKind::Image { asset: Some("pic".into()), alt: "A tiny picture".into(), caption: "Figure one".into(), width: ImageWidth::Full, decorative: false, crop: None }),
            b("btn", BlockKind::Button { label: "Learn more".into(), url: "https://example.com/more".into() }),
            b("gate", BlockKind::Continue { label: "Continue".into() }),
            b("st", BlockKind::Statement { text: "Safety first.".into() }),
            b("acc", BlockKind::Items { style: ItemsStyle::Accordion, items: vec![item("Topic A", "Details A"), item("Topic B", "Details B")] }),
            b("tabs", BlockKind::Items { style: ItemsStyle::Tabs, items: vec![item("Tab one", "First panel"), item("Tab two", "Second panel")] }),
            b("proc", BlockKind::Items { style: ItemsStyle::Process, items: vec![item("Plan", "Make a plan"), item("Do", "Do the work")] }),
            b("flash", BlockKind::Items { style: ItemsStyle::Flashcards, items: vec![item("Front", "Back")] }),
        ] },
        Section { id: "s2".into(), title: "Check".into(), summary: "Test what you learned.".into(), blocks: vec![
            b("q1", BlockKind::KnowledgeCheck(q)),
            b("end", BlockKind::Text { text: "The end.".into() }),
        ] },
    ];
    if std::env::var_os("WL_NOGATE").is_some() { p.lesson.as_mut().unwrap().sections[0].blocks.retain(|b| b.id != "gate"); }
    let out = std::path::Path::new(&out);
    match mode.as_deref() {
        Some("scorm12") => brainbend_publish_html::export_scorm(&p, brainbend_publish_html::Lms::Scorm12, out).unwrap(),
        Some("scorm2004") => brainbend_publish_html::export_scorm(&p, brainbend_publish_html::Lms::Scorm2004, out).unwrap(),
        _ => brainbend_publish_html::export(&p, out).unwrap(),
    }
}
