// A fuller Web Lesson for reviewing the published design: webdemo <out_dir> [theme] [cover.svg] [primary_hex] [secondary_hex]
use brainbend_doc::lesson::*;
use brainbend_doc::*;

fn hexc(s: &str) -> Rgba {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap();
    Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255)
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut p = Project::new_web_lesson("The Swiss Cheese Model", ProjectKind::WebLesson);
    if let Some(t) = a.get(1) { if let Some(t) = Theme::presets().into_iter().find(|x| x.name.eq_ignore_ascii_case(t)) { p.theme = t; } }
    if let Some(f) = a.get(2).filter(|f| !f.is_empty()) {
        p.assets.insert("cover".into(), Asset { name: "cover.svg".into(), mime: "image/svg+xml".into(), bytes: std::fs::read(f).unwrap(), file: None });
    }
    let b = |id: &str, kind| Block { id: id.into(), kind };
    let item = |t: &str, body: &str| Item { title: t.into(), body: body.into() };
    let mut q = Question::new_single();
    q.prompt = "What does a hole in one slice mean?".into();
    q.choices[0].text = "A weakness in one layer of defence".into();
    q.choices[1].text = "An accident that has already happened".into();
    let l = p.lesson.as_mut().unwrap();
    l.primary = a.get(3).filter(|s| !s.is_empty()).map(|s| hexc(s));
    l.secondary = a.get(4).filter(|s| !s.is_empty()).map(|s| hexc(s));
    l.cover = Some(Cover {
        subtitle: "Human factors".into(),
        description: "Why accidents rarely have a single cause, and how layered defences stop most problems before they reach people.".into(),
        image: a.get(2).filter(|f| !f.is_empty()).map(|_| "cover".to_string()),
    });
    let sec = |id: &str, title: &str, summary: &str, blocks| Section { id: id.into(), title: title.into(), summary: summary.into(), blocks };
    l.sections = vec![
        sec("s1", "Why accidents happen", "Active failures, latent conditions and why blaming one person misses the point.", vec![
            b("h1", BlockKind::Heading { text: "Rarely one cause".into(), level: 2 }),
            b("t1", BlockKind::Text { text: "Most serious accidents are the end of a chain. A **tired operator**, a *confusing label* and a skipped check line up, and the chain completes.\n\n- Active failures happen at the sharp end\n- Latent conditions sit in the system for years\n- Defences fail in small, ordinary ways".into() }),
            b("st", BlockKind::Statement { text: "Find the holes in the system, not the person in front of it.".into() }),
            b("q", BlockKind::Quote { text: "We cannot change the human condition, but we can change the conditions under which humans work.".into(), by: "James Reason".into() }),
            b("go", BlockKind::Continue { label: "Continue".into() }),
            b("t2", BlockKind::Text { text: "Layered defences each have weaknesses. The accident path only exists when the holes line up.".into() }),
        ]),
        sec("s2", "The layers of defence", "Barriers, procedures, people and organisation, and where each one leaks.", vec![
            b("a", BlockKind::Items { style: ItemsStyle::Accordion, items: vec![item("Organisational influences", "Budgets, culture and policy shape everything below."), item("Supervision", "Planning, oversight and correcting problems early."), item("Preconditions", "Fatigue, workload, tools and environment."), item("Acts", "What people do, including slips and violations.")] }),
            b("p", BlockKind::Items { style: ItemsStyle::Process, items: vec![item("Spot the hole", "Report near misses."), item("Find the cause", "Ask why until you reach the system."), item("Close it", "Fix the process, not just the person.")] }),
        ]),
        sec("s3", "Using the model", "Practical ways to find and close holes before they line up.", vec![
            b("t3", BlockKind::Text { text: "Use the model in reviews: for each layer ask *what was supposed to stop this, and why didn't it?*".into() }),
            b("tl", BlockKind::Items { style: ItemsStyle::Timeline, items: vec![item("1990", "The model is published."), item("2000", "Adopted across aviation and healthcare."), item("Today", "Used in safety reviews everywhere.")] }),
        ]),
        sec("s4", "Check your understanding", "Two quick questions to finish.", vec![
            b("k", BlockKind::KnowledgeCheck(q)),
            b("end", BlockKind::Text { text: "Thanks for completing the lesson.".into() }),
        ]),
    ];
    brainbend_publish_html::export(&p, std::path::Path::new(&a[0])).unwrap();
}
