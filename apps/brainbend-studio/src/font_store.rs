//! Fonts in the app: giving the editor the fonts a project uses, the font picker, and adding fonts from Google Fonts.
//!
//! `brainbend-fonts` knows which fonts exist (bundled, downloaded, installed). This module hands the ones in use to
//! egui, which has to be given every font it will draw with, and lets the author choose and add them.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{channel, Receiver};

// ----- giving fonts to egui -----

/// What the UI thread has given egui. It is per thread (the editor draws on one thread), which also keeps tests that each
/// run their own UI apart.
#[derive(Default)]
struct State {
    /// What egui has been given so far, so a font can be added later without losing the others.
    defs: Option<egui::FontDefinitions>,
    /// Families egui has: lowercase name -> (the name it is registered under, whether a separate bold face is registered).
    loaded: BTreeMap<String, (String, bool)>,
    /// Families something wants to draw with that egui does not have yet.
    pending: BTreeSet<String>,
    /// Families just handed to egui. It takes them up at the start of its next frame, so they are drawn with from then on.
    staged: BTreeMap<String, (String, bool)>,
}

thread_local! {
    static STATE: std::cell::RefCell<State> = std::cell::RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R { STATE.with(|s| f(&mut s.borrow_mut())) }

fn data_for(face: &brainbend_fonts::Face) -> Option<egui::FontData> {
    if let Some(b) = face.static_bytes() { return Some(egui::FontData::from_static(b)); }
    let (bytes, index) = face.data()?;
    let mut d = egui::FontData::from_owned(bytes);
    d.index = index;
    Some(d)
}

/// Adds one family to `defs`: its upright face as `Name(family)`, and for a font that is not variable its bold face as
/// `Name("family|b")`. (Bold on a variable font is a setting of the font itself; italic is slanted by egui.)
fn register(defs: &mut egui::FontDefinitions, loaded: &mut BTreeMap<String, (String, bool)>, fam: &brainbend_fonts::Family) -> bool {
    use std::sync::Arc;
    let Some(upright) = fam.face_for(false, false) else { return false };
    let Some(data) = data_for(upright) else { return false };
    let fallback: Vec<String> = {
        let mut chain = defs.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
        // Hack covers the maths operators and arrows other fonts lack (the Symbol picker offers them).
        if defs.font_data.contains_key("Hack") { chain.push("Hack".to_string()); }
        chain
    };
    let key = format!("{}#{}", fam.name, upright.file_name);
    defs.font_data.insert(key.clone(), Arc::new(data));
    let mut chain = vec![key];
    chain.extend(fallback.iter().cloned());
    defs.families.insert(egui::FontFamily::Name(fam.name.clone().into()), chain);
    let mut has_bold = false;
    if !fam.is_variable() {
        if let Some(bold) = fam.face_for(true, false).filter(|f| f.weight >= 600 && !f.italic) {
            if let Some(d) = data_for(bold) {
                let bkey = format!("{}#{}", fam.name, bold.file_name);
                defs.font_data.insert(bkey.clone(), Arc::new(d));
                let mut chain = vec![bkey];
                chain.extend(fallback);
                defs.families.insert(egui::FontFamily::Name(format!("{}|b", fam.name).into()), chain);
                has_bold = true;
            }
        }
    }
    loaded.insert(fam.name.to_lowercase(), (fam.name.clone(), has_bold));
    true
}

/// Starts the editor's fonts: the icon font, the interface fonts and the two families every project can use.
pub(super) fn install(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    defs.font_data.insert("icons".into(), std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../../assets/icons/Phosphor.ttf"))));
    defs.families.insert(ribbon::icons(), vec!["icons".into()]);
    with(|st| {
        st.loaded.clear();
        st.pending.clear();
        for name in ["Montserrat", "Open Sans"] {
            if let Some(fam) = brainbend_fonts::lookup(name) { register(&mut defs, &mut st.loaded, &fam); }
        }
        st.defs = Some(defs.clone());
    });
    ctx.set_fonts(defs);
}

/// Asks for a family to be given to egui before it is next drawn with.
pub(super) fn want(name: &str) {
    if name.is_empty() { return; }
    let key = name.to_lowercase();
    with(|st| {
        if !st.loaded.contains_key(&key) && !st.staged.contains_key(&key) && brainbend_fonts::available(name) { st.pending.insert(name.to_string()); }
    });
}

/// Gives egui the families asked for. Call at the start of a frame; text drawn with them appears the frame after.
pub(super) fn apply_pending(ctx: &egui::Context) {
    let fonts = with(|st| {
        // What was handed over last frame is now in egui's hands.
        let staged = std::mem::take(&mut st.staged);
        st.loaded.extend(staged);
        if st.pending.is_empty() { return None; }
        let names: Vec<String> = std::mem::take(&mut st.pending).into_iter().collect();
        let State { defs, staged, .. } = st;
        let defs = defs.as_mut()?;
        let mut any = false;
        for name in names {
            if let Some(fam) = brainbend_fonts::lookup(&name) { any |= register(defs, staged, &fam); }
        }
        any.then(|| defs.clone())
    });
    if let Some(defs) = fonts { ctx.set_fonts(defs); ctx.request_repaint(); }
}

/// Is something still waiting to be given to egui (so the screen needs another frame)?
pub(super) fn has_pending() -> bool { with(|st| !st.pending.is_empty() || !st.staged.is_empty()) }

/// The egui family for a font name, or Open Sans for one that is not available (yet).
pub(super) fn family(name: &str) -> egui::FontFamily {
    let found = with(|st| st.loaded.get(&name.to_lowercase()).map(|(canon, _)| canon.clone()));
    if let Some(canon) = found { return egui::FontFamily::Name(canon.as_str().into()); }
    want(name);
    egui::FontFamily::Name("Open Sans".into())
}

/// The egui family for text that may be bold: a font with a separate bold file uses it; a variable font stays as it is and
/// takes its weight from the `wght` setting.
pub(super) fn family_styled(name: &str, bold: bool) -> egui::FontFamily {
    if bold {
        let canon = with(|st| match st.loaded.get(&name.to_lowercase()) { Some((canon, true)) => Some(canon.clone()), _ => None });
        if let Some(canon) = canon { return egui::FontFamily::Name(format!("{canon}|b").into()); }
    }
    family(name)
}

// ----- the picker -----

/// Marks the "More fonts…" window as wanted; `App::show` opens it.
const MORE_FLAG: &str = "fonts-more-requested";

fn source_note(src: brainbend_fonts::Source) -> &'static str {
    match src {
        brainbend_fonts::Source::Bundled => "",
        brainbend_fonts::Source::Downloaded => "from Google Fonts",
        brainbend_fonts::Source::System => "installed on this computer",
    }
}

enum Row {
    Header(&'static str, &'static str),
    Font(String),
}

/// A font drop-down: the bundled families and any added from Google Fonts, each drawn in its own face, then the fonts
/// installed on this computer. Returns whether the choice changed.
pub(super) fn font_menu(ui: &mut egui::Ui, id: &str, current: &mut String, width: f32) -> bool {
    let mut changed = false;
    let known = brainbend_fonts::available(current);
    want(current);
    let label = if known || current.is_empty() { current.clone() } else { format!("{current} (not installed)") };
    egui::ComboBox::from_id_salt(id).width(width).selected_text(label).height(460.0).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show_ui(ui, |ui| {
        ui.set_min_width(280.0);
        let q_id = egui::Id::new(("font-menu-query", id));
        let mut query: String = ui.data(|d| d.get_temp(q_id)).unwrap_or_default();
        let edit = ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search fonts").desired_width(f32::INFINITY));
        if edit.changed() { ui.data_mut(|d| d.insert_temp(q_id, query.clone())); }
        let q = query.trim().to_lowercase();
        let all = brainbend_fonts::families();
        let mut rows: Vec<Row> = vec![];
        for (heading, note, src) in [
            ("BrainBend fonts", "Included with BrainBend Studio. They travel with your lesson.", brainbend_fonts::Source::Bundled),
            ("Added from Google Fonts", "Open-licensed. They travel with your lesson.", brainbend_fonts::Source::Downloaded),
            ("Installed on this computer", "Shown here for editing. Embedded in a published lesson only where the font's license allows it.", brainbend_fonts::Source::System),
        ] {
            let names: Vec<&String> = all.iter().filter(|(n, s)| *s == src && (q.is_empty() || n.to_lowercase().contains(&q))).map(|(n, _)| n).collect();
            if names.is_empty() { continue; }
            rows.push(Row::Header(heading, note));
            rows.extend(names.into_iter().map(|n| Row::Font(n.clone())));
        }
        ui.add_space(2.0);
        egui::ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show_rows(ui, 26.0, rows.len(), |ui, range| {
          ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            for r in &rows[range] {
                match r {
                    Row::Header(h, note) => {
                        ui.add_space(2.0);
                        ui.add(egui::Label::new(egui::RichText::new(*h).strong().size(11.5).weak()).truncate()).on_hover_text(*note);
                    }
                    Row::Font(name) => {
                        let src = brainbend_fonts::source_of(name);
                        // Only families that are not installed system-wide are drawn in their own face: previewing every
                        // installed font would load them all.
                        let rich = if src != Some(brainbend_fonts::Source::System) {
                            want(name);
                            egui::RichText::new(name).family(family(name)).size(15.0)
                        } else {
                            egui::RichText::new(name).size(14.0)
                        };
                        let resp = ui.add(egui::Button::selectable(name.eq_ignore_ascii_case(current), rich).min_size(egui::vec2(0.0, 24.0)));
                        if resp.clicked() { *current = name.clone(); changed = true; ui.close(); }
                        if let Some(s) = src { let n = source_note(s); if !n.is_empty() { resp.on_hover_text(n); } }
                    }
                }
            }
          });
        });
        ui.separator();
        if ui.button("More fonts from Google Fonts…").clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(MORE_FLAG), true));
            ui.close();
        }
    });
    changed
}

// ----- adding fonts from Google Fonts -----

#[derive(Clone)]
pub struct Listed {
    pub family: String,
    pub category: brainbend_fonts::Category,
    /// Google's popularity rank (1 is the most used).
    pub rank: u32,
}

type Reply<T> = Receiver<Result<T, String>>;

/// The Google Fonts window's state: the catalogue, the search, and the downloads under way.
#[derive(Default)]
pub struct FontStore {
    pub open: bool,
    catalogue: Option<Vec<Listed>>,
    loading: Option<Reply<Vec<Listed>>>,
    error: Option<String>,
    query: String,
    category: Option<brainbend_fonts::Category>,
    jobs: Vec<(String, Reply<String>)>,
    failed: BTreeMap<String, String>,
}

fn store_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(std::path::PathBuf::from(home).join(".brainbend-studio").join("fonts"))
}

fn catalogue_cache() -> Option<std::path::PathBuf> { Some(store_dir()?.join("google-fonts.json")) }

fn http_get(url: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(60))).build().into();
    let mut reply = agent.get(url).header("User-Agent", "Mozilla/5.0 BrainBendStudio").call().map_err(|e| e.to_string())?;
    reply.body_mut().with_config().limit(40 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())
}

/// Reads Google's font list (`familyMetadataList`), most popular first.
fn parse_catalogue(text: &str) -> Result<Vec<Listed>, String> {
    let body = text.trim_start_matches(")]}'").trim_start();
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| format!("could not read the font list: {e}"))?;
    let list = v.get("familyMetadataList").and_then(|l| l.as_array()).ok_or("the font list was not in the expected form")?;
    let mut out: Vec<Listed> = list.iter().filter_map(|f| {
        Some(Listed {
            family: f.get("family")?.as_str()?.to_string(),
            category: brainbend_fonts::Category::from_google(f.get("category").and_then(|c| c.as_str()).unwrap_or("")),
            rank: f.get("popularity").and_then(|p| p.as_u64()).unwrap_or(9999) as u32,
        })
    }).collect();
    out.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.family.cmp(&b.family)));
    Ok(out)
}

/// Picks which files of a family to keep: the variable fonts if there are any, else Regular, Bold, Italic and Bold Italic.
fn choose_files(names: &[String]) -> Vec<String> {
    let flat: Vec<&String> = names.iter().filter(|n| !n.contains('/') && (n.ends_with(".ttf") || n.ends_with(".otf"))).collect();
    let variable: Vec<String> = flat.iter().filter(|n| n.contains("VariableFont")).map(|n| n.to_string()).collect();
    if !variable.is_empty() { return variable; }
    let wanted: Vec<String> = flat.iter().filter(|n| ["-Regular.", "-Bold.", "-Italic.", "-BoldItalic."].iter().any(|s| n.contains(s))).map(|n| n.to_string()).collect();
    if !wanted.is_empty() { return wanted; }
    flat.first().map(|n| vec![n.to_string()]).unwrap_or_default()
}

/// Downloads a family into the fonts folder and loads it. Returns the family's name.
fn download_family(listed: &Listed) -> Result<String, String> {
    let dir = store_dir().ok_or("there is no home folder to keep fonts in")?.join(&listed.family);
    let url = format!("https://fonts.google.com/download/list?family={}", listed.family.replace(' ', "%20"));
    let text = String::from_utf8(http_get(&url)?).map_err(|_| "unreadable reply".to_string())?;
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches(")]}'").trim_start()).map_err(|e| format!("could not read the download list: {e}"))?;
    let manifest = v.get("manifest").ok_or("no files listed")?;
    let refs = manifest.get("fileRefs").and_then(|r| r.as_array()).ok_or("no font files listed")?;
    let names: Vec<String> = refs.iter().filter_map(|r| r.get("filename")?.as_str().map(String::from)).collect();
    let chosen = choose_files(&names);
    if chosen.is_empty() { return Err("no font files found".into()); }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for r in refs {
        let (Some(name), Some(url)) = (r.get("filename").and_then(|n| n.as_str()), r.get("url").and_then(|u| u.as_str())) else { continue };
        if !chosen.iter().any(|c| c == name) { continue; }
        let bytes = http_get(url)?;
        std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    }
    // The license text travels with the font.
    if let Some(files) = manifest.get("files").and_then(|f| f.as_array()) {
        for f in files {
            let (Some(name), Some(contents)) = (f.get("filename").and_then(|n| n.as_str()), f.get("contents").and_then(|c| c.as_str())) else { continue };
            let upper = name.to_uppercase();
            if upper.starts_with("OFL") || upper.starts_with("LICENSE") || upper.starts_with("UFL") {
                let _ = std::fs::write(dir.join("OFL.txt"), contents.replace("\r\n", "\n"));
            }
        }
    }
    let _ = std::fs::write(dir.join("meta.json"), serde_json::json!({ "category": listed.category.google_name() }).to_string());
    brainbend_fonts::load_family_dir(&dir).ok_or_else(|| "the downloaded files could not be read".to_string())
}

/// Loads the fonts added earlier and starts reading the installed fonts in the background.
pub(super) fn start(ctx: &egui::Context) {
    if let Some(dir) = store_dir() { brainbend_fonts::load_downloaded(&dir); }
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        brainbend_fonts::scan_system();
        ctx.request_repaint();
    });
}

impl FontStore {
    /// A window already open on a given list, for tests that must not reach the network.
    #[cfg(test)]
    pub(super) fn open_with(list: Vec<(&str, brainbend_fonts::Category)>) -> Self {
        let catalogue = list.into_iter().enumerate().map(|(i, (f, c))| Listed { family: f.to_string(), category: c, rank: i as u32 + 1 }).collect();
        Self { open: true, catalogue: Some(catalogue), ..Default::default() }
    }

    fn start_catalogue(&mut self, ctx: &egui::Context, refresh: bool) {
        if !refresh {
            if let Some(text) = catalogue_cache().and_then(|p| std::fs::read_to_string(p).ok()) {
                if let Ok(list) = parse_catalogue(&text) { self.catalogue = Some(list); return; }
            }
        }
        let (tx, rx) = channel();
        self.loading = Some(rx);
        self.error = None;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = http_get("https://fonts.google.com/metadata/fonts").and_then(|b| String::from_utf8(b).map_err(|_| "unreadable reply".to_string())).and_then(|text| {
                let list = parse_catalogue(&text)?;
                if let Some(p) = catalogue_cache() { if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); } let _ = std::fs::write(p, &text); }
                Ok(list)
            });
            let _ = tx.send(r);
            ctx.request_repaint();
        });
    }

    fn poll(&mut self) {
        if let Some(rx) = &self.loading {
            if let Ok(r) = rx.try_recv() {
                self.loading = None;
                match r { Ok(l) => self.catalogue = Some(l), Err(e) => self.error = Some(e) }
            }
        }
        let mut done = vec![];
        for (i, (name, rx)) in self.jobs.iter().enumerate() {
            if let Ok(r) = rx.try_recv() { done.push((i, name.clone(), r)); }
        }
        for (i, name, r) in done.into_iter().rev() {
            self.jobs.remove(i);
            match r { Ok(fam) => { want(&fam); self.failed.remove(&name); } Err(e) => { self.failed.insert(name, e); } }
        }
    }

    fn add(&mut self, ctx: &egui::Context, listed: Listed) {
        if self.jobs.iter().any(|(n, _)| *n == listed.family) { return; }
        let (tx, rx) = channel();
        let name = listed.family.clone();
        self.failed.remove(&name);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(download_family(&listed));
            ctx.request_repaint();
        });
        self.jobs.push((name, rx));
    }
}

impl App {
    /// The "More fonts from Google Fonts" window.
    pub(super) fn fonts_window(&mut self, ctx: &egui::Context) {
        if ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::new(MORE_FLAG))).unwrap_or(false) {
            self.font_store.open = true;
            if self.font_store.catalogue.is_none() && self.font_store.loading.is_none() { self.font_store.start_catalogue(ctx, false); }
        }
        if !self.font_store.open { return; }
        self.font_store.poll();
        let mut open = true;
        let mut add: Option<Listed> = None;
        let mut refresh = false;
        egui::Window::new("More fonts from Google Fonts").id(egui::Id::new("google-fonts")).open(&mut open).collapsible(false).resizable(true).default_size([520.0, 560.0]).show(ctx, |ui| {
            let store = &mut self.font_store;
            ui.label("Add open-licensed fonts from Google Fonts. They are saved on this computer, show up in every font menu, and travel with the lessons you publish.");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut store.query).hint_text("Search").desired_width(220.0));
                egui::ComboBox::from_id_salt("gf-category").selected_text(store.category.map(|c| c.google_name()).unwrap_or("All styles")).show_ui(ui, |ui| {
                    ui.selectable_value(&mut store.category, None, "All styles");
                    for c in [brainbend_fonts::Category::Sans, brainbend_fonts::Category::Serif, brainbend_fonts::Category::Display, brainbend_fonts::Category::Handwriting, brainbend_fonts::Category::Mono] {
                        ui.selectable_value(&mut store.category, Some(c), c.google_name());
                    }
                });
                if ui.button("Refresh").on_hover_text("Fetch the latest list from Google Fonts").clicked() { refresh = true; }
            });
            ui.add_space(6.0);
            if store.loading.is_some() {
                ui.horizontal(|ui| { ui.add(egui::Spinner::new()); ui.label("Getting the list of fonts…"); });
                return;
            }
            if let Some(e) = &store.error {
                ui.colored_label(ui.visuals().error_fg_color, format!("Could not reach Google Fonts: {e}"));
                if ui.button("Try again").clicked() { refresh = true; }
                return;
            }
            let Some(list) = &store.catalogue else { return };
            let q = store.query.trim().to_lowercase();
            let shown: Vec<&Listed> = list.iter().filter(|f| (q.is_empty() || f.family.to_lowercase().contains(&q)) && store.category.is_none_or(|c| c == f.category)).collect();
            ui.weak(format!("{} fonts", shown.len()));
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 30.0, shown.len(), |ui, range| {
                for f in &shown[range] {
                    ui.horizontal(|ui| {
                        ui.set_height(28.0);
                        let have = brainbend_fonts::source_of(&f.family).is_some_and(|s| s != brainbend_fonts::Source::System);
                        ui.allocate_ui_with_layout(egui::vec2(210.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(egui::Label::new(egui::RichText::new(&f.family).size(14.5)).truncate());
                        });
                        ui.weak(f.category.google_name());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if store.jobs.iter().any(|(n, _)| *n == f.family) {
                                ui.add(egui::Spinner::new());
                            } else if have {
                                ui.weak("Added");
                            } else {
                                if let Some(err) = store.failed.get(&f.family) { ui.colored_label(ui.visuals().error_fg_color, "Failed").on_hover_text(err); }
                                if ui.button("Add").clicked() { add = Some((*f).clone()); }
                            }
                        });
                    });
                }
            });
        });
        if refresh { self.font_store.start_catalogue(ctx, true); }
        if let Some(f) = add { self.font_store.add(ctx, f); }
        if !open { self.font_store.open = false; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_is_read_most_popular_first_with_google_prefix_removed() {
        let text = r#")]}'
{"familyMetadataList":[{"family":"Zeta","category":"Serif","popularity":2},{"family":"Alpha","category":"Monospace","popularity":1},{"family":"NoRank","category":"Display"}]}"#;
        let l = parse_catalogue(text).unwrap();
        assert_eq!(l.iter().map(|f| f.family.as_str()).collect::<Vec<_>>(), ["Alpha", "Zeta", "NoRank"]);
        assert_eq!(l[0].category, brainbend_fonts::Category::Mono);
        assert_eq!(l[1].category, brainbend_fonts::Category::Serif);
        assert!(parse_catalogue("not json").is_err());
    }

    /// Talks to Google: `cargo test -p brainbend-studio -- --ignored google_fonts`.
    #[test]
    #[ignore]
    fn google_fonts_catalogue_and_download_work_for_real() {
        let text = String::from_utf8(http_get("https://fonts.google.com/metadata/fonts").unwrap()).unwrap();
        let list = parse_catalogue(&text).unwrap();
        assert!(list.len() > 1000 && list[0].rank <= list[1].rank);
        let home = std::env::temp_dir().join(format!("bb-gf-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("HOME", &home);
        let pick = list.iter().find(|f| f.family == "Pacifico").unwrap();
        let name = download_family(pick).unwrap();
        assert_eq!(name, "Pacifico");
        let fam = brainbend_fonts::lookup("Pacifico").unwrap();
        assert_eq!(fam.source, brainbend_fonts::Source::Downloaded);
        assert!(fam.license.as_deref().is_some_and(|l| l.contains("SIL OPEN FONT LICENSE")));
        assert_eq!(fam.category, brainbend_fonts::Category::Handwriting);
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn a_download_keeps_variable_fonts_else_the_four_basic_faces() {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(choose_files(&v(&["Inter-VariableFont_opsz,wght.ttf", "Inter-Italic-VariableFont_opsz,wght.ttf", "static/Inter_18pt-Regular.ttf"])),
            v(&["Inter-VariableFont_opsz,wght.ttf", "Inter-Italic-VariableFont_opsz,wght.ttf"]));
        assert_eq!(choose_files(&v(&["Lato-Thin.ttf", "Lato-Regular.ttf", "Lato-Bold.ttf", "Lato-Italic.ttf", "Lato-BoldItalic.ttf"])),
            v(&["Lato-Regular.ttf", "Lato-Bold.ttf", "Lato-Italic.ttf", "Lato-BoldItalic.ttf"]));
        assert_eq!(choose_files(&v(&["BebasNeue-Regular.ttf"])), v(&["BebasNeue-Regular.ttf"]));
    }
}
