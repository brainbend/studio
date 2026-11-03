// A project that uses the player settings, for the player tests and screenshots: playerui <out_dir> [left|right|off] [icons]
//
// Three slides with notes, a sidebar with Menu and Notes tabs, a title, a logo and a banner photo.
use brainbend_doc::*;

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let side = std::env::args().nth(2).unwrap_or_else(|| "left".into());
    let mut p = Project::new("Safety Basics");
    for (i, (name, notes)) in [("Welcome", "Greet the learners.\nExplain the course."), ("Hazards", ""), ("Wrap up", "Summarise the three rules.")].iter().enumerate() {
        let mut s = Slide::new(&format!("s{i}"), name, 1280.0, 720.0);
        let mut t = Object::new(&format!("t{i}"), "Text", Kind::Text, 100.0, 100.0, 800.0, 120.0);
        t.text = format!("Slide {}: {}", i + 1, name);
        s.layers[0].objects = vec![t];
        s.notes = notes.to_string();
        p.slides.push(s);
    }
    p.show_menu = true;
    p.assets.insert("logo".into(), Asset { name: "logo.svg".into(), mime: "image/svg+xml".into(), bytes: br##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="40"><rect width="120" height="40" rx="6" fill="#e87722"/><text x="60" y="26" font-family="Arial" font-size="18" font-weight="700" text-anchor="middle" fill="#fff">ACME</text></svg>"##.to_vec(), file: None });
    p.resources = vec![
        Resource { title: "Safety handbook".into(), url: "https://example.com/handbook".into(), asset: None },
        Resource { title: "Unsafe link".into(), url: "javascript:alert(1)".into(), asset: None },
    ];
    p.glossary = vec![
        GlossaryTerm { term: "PPE".into(), definition: "Personal protective equipment.".into() },
        GlossaryTerm { term: "Hazard".into(), definition: "Anything that can cause harm.".into() },
    ];
    p.assets.insert("banner".into(), Asset { name: "banner.svg".into(), mime: "image/svg+xml".into(), bytes: br##"<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="300" viewBox="0 0 1600 300"><defs><linearGradient id="g" x1="0" x2="1"><stop offset="0" stop-color="#03539e"/><stop offset="1" stop-color="#e25b1c"/></linearGradient></defs><rect width="1600" height="300" fill="url(#g)"/></svg>"##.to_vec(), file: None });
    p.player.logo = Some("logo".into());
    p.player.banner = Some("banner".into());
    p.player.show_title = true;
    p.player.tab_notes = true;
    p.player.tab_glossary = true;
    p.player.tab_resources = true;
    p.player.search = true;
    p.player.sidebar = match side.as_str() { "right" => SidebarSide::Right, "off" => SidebarSide::Off, _ => SidebarSide::Left };
    if std::env::args().nth(3).as_deref() == Some("icons") {
        p.player.buttons = ButtonStyle::IconText;
        p.player.controls = PlayerControls { volume: true, captions: true, speed: true, fullscreen: true };
        p.player.theme = PlayerTheme::Light;
        p.player.accent = Rgba(232, 119, 34, 255);
    }
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
