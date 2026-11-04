//! The fonts BrainBend Studio can use, wherever they come from, and how they travel with a published lesson.
//!
//! Three sources feed one library:
//! - **Bundled**: open-licensed families compiled into the app (`assets/fonts/<Family>/`). Always embeddable.
//! - **Downloaded**: families fetched from Google Fonts into `~/.brainbend-studio/fonts/`. Open-licensed, embeddable.
//! - **System**: whatever is installed on this computer. Embedded in an export only where the font's own embedding
//!   permission allows it; otherwise the export names the font with a fallback and the learner's own copy is used.
//!
//! The library is process-wide so the editor, the exporters and the importers all see the same fonts.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

mod generated {
    include!(concat!(env!("OUT_DIR"), "/bundled.rs"));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Bundled,
    Downloaded,
    System,
}

/// What a family looks like, for the generic fallback named after it in CSS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Sans,
    Serif,
    Mono,
    Display,
    Handwriting,
}

impl Category {
    pub fn css_generic(self) -> &'static str {
        match self {
            Category::Sans => "sans-serif",
            Category::Serif => "serif",
            Category::Mono => "monospace",
            Category::Display => "sans-serif",
            Category::Handwriting => "cursive",
        }
    }

    /// Google Fonts' category names.
    pub fn from_google(name: &str) -> Category {
        match name {
            "Serif" => Category::Serif,
            "Monospace" => Category::Mono,
            "Display" => Category::Display,
            "Handwriting" => Category::Handwriting,
            _ => Category::Sans,
        }
    }

    pub fn google_name(self) -> &'static str {
        match self {
            Category::Sans => "Sans Serif",
            Category::Serif => "Serif",
            Category::Mono => "Monospace",
            Category::Display => "Display",
            Category::Handwriting => "Handwriting",
        }
    }
}

#[derive(Clone)]
enum Data {
    Static(&'static [u8]),
    File(PathBuf, u32),
}

/// One font file of a family: a weight and slant, or a variable font that spans a range of weights.
#[derive(Clone)]
pub struct Face {
    pub file_name: String,
    pub weight: u16,
    pub italic: bool,
    /// For a variable font: the weights it spans.
    pub weight_range: Option<(u16, u16)>,
    /// For a variable font with a width axis: the widths it spans, in percent.
    pub width_range: Option<(u16, u16)>,
    data: Data,
}

impl Face {
    /// The font file's bytes and its index within a collection, or `None` if the file cannot be read.
    pub fn data(&self) -> Option<(Vec<u8>, u32)> {
        match &self.data {
            Data::Static(b) => Some((b.to_vec(), 0)),
            Data::File(p, i) => std::fs::read(p).ok().map(|b| (b, *i)),
        }
    }

    /// The bytes without copying, for the faces compiled into the app.
    pub fn static_bytes(&self) -> Option<&'static [u8]> {
        match &self.data { Data::Static(b) => Some(b), Data::File(..) => None }
    }

    /// May this font be embedded in a published lesson? Judged by the permission bits in the font file itself.
    /// "Installable" and "editable" embedding are allowed; "restricted" and "preview and print" are not.
    pub fn embeddable(&self) -> bool {
        let Some((bytes, index)) = self.data() else { return false };
        match ttf_parser::Face::parse(&bytes, index) {
            Ok(f) => match f.permissions() {
                None => true,
                Some(p) => matches!(p, ttf_parser::Permissions::Installable | ttf_parser::Permissions::Editable),
            },
            Err(_) => false,
        }
    }
}

#[derive(Clone)]
pub struct Family {
    pub name: String,
    pub source: Source,
    pub category: Category,
    pub faces: Vec<Face>,
    /// The font's license text (the SIL Open Font License for bundled and downloaded families).
    pub license: Option<Arc<str>>,
}

impl Family {
    /// Is this family one whose license lets it travel with a lesson without a second look?
    pub fn free(&self) -> bool { self.source != Source::System }

    /// The face to draw `bold`/`italic` text with: an exact match if there is one, else the closest.
    pub fn face_for(&self, bold: bool, italic: bool) -> Option<&Face> {
        let want = if bold { 700i32 } else { 400 };
        self.faces.iter().min_by_key(|f| {
            let slant = if f.italic == italic { 0 } else { 10_000 };
            let weight = match f.weight_range {
                Some((lo, hi)) if (lo as i32..=hi as i32).contains(&want) => 0,
                Some((lo, hi)) => (lo as i32 - want).abs().min((hi as i32 - want).abs()),
                None => (f.weight as i32 - want).abs(),
            };
            // A variable face is preferred when it can do the weight: bold needs no second file.
            slant + weight * 2 + if f.weight_range.is_some() { 0 } else { 1 }
        })
    }

    /// Does one of the faces carry the weight itself (a variable font), so that bold needs no separate file?
    pub fn is_variable(&self) -> bool { self.faces.iter().any(|f| f.weight_range.is_some()) }
}

struct Library {
    families: BTreeMap<String, Family>,
}

fn key(name: &str) -> String { name.trim().to_lowercase() }

fn library() -> &'static RwLock<Library> {
    static LIB: OnceLock<RwLock<Library>> = OnceLock::new();
    LIB.get_or_init(|| {
        let mut lib = Library { families: BTreeMap::new() };
        for (name, files, license) in generated::BUNDLED {
            let faces: Vec<Face> = files.iter().filter_map(|(file, bytes)| face_from_bytes(file, Data::Static(bytes), bytes, 0)).collect();
            if faces.is_empty() { continue; }
            lib.families.insert(key(name), Family {
                name: name.to_string(),
                source: Source::Bundled,
                category: bundled_category(name),
                faces,
                license: if license.is_empty() { None } else { Some(Arc::from(*license)) },
            });
        }
        RwLock::new(lib)
    })
}

fn bundled_category(name: &str) -> Category {
    match name {
        "Libre Baskerville" | "Playfair Display" | "Lora" | "Source Serif 4" => Category::Serif,
        "JetBrains Mono" | "Source Code Pro" => Category::Mono,
        "Oswald" | "Bebas Neue" => Category::Display,
        "Caveat" => Category::Handwriting,
        _ => Category::Sans,
    }
}

/// Reads a font file's weight, slant and variable axes. `None` if it is not a usable font.
fn face_from_bytes(file: &str, data: Data, bytes: &[u8], index: u32) -> Option<Face> {
    let f = ttf_parser::Face::parse(bytes, index).ok()?;
    let mut weight_range = None;
    let mut width_range = None;
    for axis in f.variation_axes() {
        if axis.tag == ttf_parser::Tag::from_bytes(b"wght") {
            weight_range = Some((axis.min_value.round() as u16, axis.max_value.round() as u16));
        } else if axis.tag == ttf_parser::Tag::from_bytes(b"wdth") {
            width_range = Some((axis.min_value.round() as u16, axis.max_value.round() as u16));
        }
    }
    Some(Face { file_name: file.to_string(), weight: f.weight().to_number(), italic: f.is_italic(), weight_range, width_range, data })
}

/// A family by name (case does not matter), or `None` if no font of that name is available.
pub fn lookup(name: &str) -> Option<Family> {
    library().read().ok()?.families.get(&key(name)).cloned()
}

/// Is a font of this name available here (bundled, downloaded or installed)?
pub fn available(name: &str) -> bool { lookup(name).is_some() }

/// Every family, by name, with where it comes from.
pub fn families() -> Vec<(String, Source)> {
    let Ok(lib) = library().read() else { return vec![] };
    let mut all: Vec<(String, Source)> = lib.families.values().map(|f| (f.name.clone(), f.source)).collect();
    all.sort_by_key(|a| a.0.to_lowercase());
    all
}

/// Which source a family comes from.
pub fn source_of(name: &str) -> Option<Source> { lookup(name).map(|f| f.source) }

/// Reads the families installed on this computer. Takes a moment; the app does it in the background.
/// Families already bundled or downloaded keep their place: a system copy never replaces them.
pub fn scan_system() {
    scan_system_inner();
    SCANNED.store(true, std::sync::atomic::Ordering::Release);
}

static SCANNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Has the installed-font scan finished? Until then a font may look missing only because it has not been read yet.
pub fn system_scanned() -> bool { SCANNED.load(std::sync::atomic::Ordering::Acquire) }

fn scan_system_inner() {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut found: BTreeMap<String, Family> = BTreeMap::new();
    for info in db.faces() {
        let Some((name, _)) = info.families.first() else { continue };
        if name.starts_with('.') || name.is_empty() { continue; }
        let (path, index) = match &info.source {
            fontdb::Source::File(p) => (p.clone(), info.index),
            fontdb::Source::SharedFile(p, _) => (p.clone(), info.index),
            fontdb::Source::Binary(_) => continue,
        };
        let weight = info.weight.0;
        let italic = !matches!(info.style, fontdb::Style::Normal);
        let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let fam = found.entry(key(name)).or_insert_with(|| Family {
            name: name.clone(),
            source: Source::System,
            category: if info.monospaced { Category::Mono } else { Category::Sans },
            faces: vec![],
            license: None,
        });
        fam.faces.push(Face { file_name, weight, italic, weight_range: None, width_range: None, data: Data::File(path, index) });
    }
    if let Ok(mut lib) = library().write() {
        for (k, f) in found {
            lib.families.entry(k).or_insert(f);
        }
    }
}

/// Loads the families saved by the Google Fonts download (one folder per family) from `dir`.
pub fn load_downloaded(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        if e.path().is_dir() { load_family_dir(&e.path()); }
    }
}

/// Adds (or replaces) the family saved in one folder: font files, `OFL.txt` and `meta.json` (`{"category": ...}`).
pub fn load_family_dir(folder: &Path) -> Option<String> {
    let name = folder.file_name()?.to_string_lossy().to_string();
    let mut faces = vec![];
    for e in std::fs::read_dir(folder).ok()?.flatten() {
        let p = e.path();
        if !matches!(p.extension().and_then(|x| x.to_str()), Some("ttf" | "otf")) { continue; }
        let Ok(bytes) = std::fs::read(&p) else { continue };
        let file = p.file_name()?.to_string_lossy().to_string();
        if let Some(f) = face_from_bytes(&file, Data::File(p.clone(), 0), &bytes, 0) { faces.push(f); }
    }
    if faces.is_empty() { return None; }
    faces.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    let license = std::fs::read_to_string(folder.join("OFL.txt")).ok().map(|t| Arc::from(t.as_str()));
    let category = std::fs::read_to_string(folder.join("meta.json")).ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("category").and_then(|c| c.as_str()).map(Category::from_google))
        .unwrap_or(Category::Sans);
    let family = Family { name: name.clone(), source: Source::Downloaded, category, faces, license };
    if let Ok(mut lib) = library().write() {
        // A bundled family of the same name stays as it is.
        let k = key(&name);
        if lib.families.get(&k).is_some_and(|f| f.source == Source::Bundled) { return Some(name); }
        lib.families.insert(k, family);
    }
    Some(name)
}

/// The CSS `font-family` value for a family: its name, then a generic fallback of the right kind.
pub fn css_stack(name: &str) -> String {
    let generic = lookup(name).map(|f| f.category.css_generic()).unwrap_or("sans-serif");
    format!("'{}',{}", name.replace('\'', ""), generic)
}

/// What an export carries for the fonts a lesson uses.
#[derive(Default)]
pub struct Embedded {
    /// `@font-face` rules for the fonts that travel with the lesson.
    pub css: String,
    /// Files to write, relative to the export folder: the fonts and their license texts.
    pub files: Vec<(String, Vec<u8>)>,
    /// Families embedded.
    pub embedded: Vec<String>,
    /// Families named in the lesson that do not travel: not installed here, or not allowed to be embedded.
    /// Learners see these only if they have the font themselves.
    pub fallback: Vec<String>,
}

fn slug(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().trim_matches('-').to_string()
}

/// Gathers the fonts for `used` families: `@font-face` rules pointing at `<prefix><file>` and the files themselves.
pub fn embed(used: &BTreeSet<String>, prefix: &str) -> Embedded {
    let mut out = Embedded::default();
    for name in used {
        let Some(fam) = lookup(name) else { out.fallback.push(name.clone()); continue };
        let allowed = fam.free() || fam.faces.iter().all(|f| f.embeddable());
        if !allowed { out.fallback.push(name.clone()); continue; }
        let mut wrote_any = false;
        for face in &fam.faces {
            let Some((bytes, _)) = face.data() else { continue };
            let file = format!("{}-{}", slug(&fam.name), face.file_name.replace(['[', ']', ','], "-"));
            let mut rule = format!("@font-face{{font-family:'{}';src:url({}{});font-style:{};", fam.name.replace('\'', ""), prefix, file, if face.italic { "italic" } else { "normal" });
            match face.weight_range {
                Some((lo, hi)) => rule.push_str(&format!("font-weight:{lo} {hi};")),
                None => rule.push_str(&format!("font-weight:{};", face.weight)),
            }
            if let Some((lo, hi)) = face.width_range { rule.push_str(&format!("font-stretch:{lo}% {hi}%;")); }
            rule.push_str("font-display:swap}\n");
            out.css.push_str(&rule);
            out.files.push((format!("fonts/{file}"), bytes));
            wrote_any = true;
        }
        if !wrote_any { out.fallback.push(name.clone()); continue; }
        if let Some(lic) = &fam.license {
            out.files.push((format!("fonts/{}-license.txt", slug(&fam.name)), lic.as_bytes().to_vec()));
        }
        out.embedded.push(fam.name.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_library_has_the_fonts_the_app_ships() {
        for name in ["Montserrat", "Open Sans", "Inter", "Roboto", "Lato", "Playfair Display", "JetBrains Mono"] {
            assert!(available(name), "{name} is bundled");
        }
        assert!(available("open sans"), "names ignore case");
        assert_eq!(source_of("Inter"), Some(Source::Bundled));
        assert!(!available("Definitely Not A Font"));
    }

    #[test]
    fn variable_fonts_report_their_weights_and_static_ones_pick_the_nearest_face() {
        let inter = lookup("Inter").unwrap();
        assert!(inter.is_variable());
        let up = inter.faces.iter().find(|f| !f.italic).unwrap();
        assert!(up.weight_range.is_some_and(|(lo, hi)| lo <= 400 && hi >= 700));
        let lato = lookup("Lato").unwrap();
        assert!(!lato.is_variable());
        assert_eq!(lato.face_for(true, false).unwrap().weight, 700);
        assert_eq!(lato.face_for(false, false).unwrap().weight, 400);
        assert!(lato.face_for(false, true).unwrap().italic);
        assert!(lato.face_for(true, true).unwrap().italic && lato.face_for(true, true).unwrap().weight == 700);
    }

    #[test]
    fn embedding_writes_rules_files_and_licenses_for_the_fonts_used() {
        let used: BTreeSet<String> = ["Inter", "Not Installed Anywhere"].iter().map(|s| s.to_string()).collect();
        let e = embed(&used, "fonts/");
        assert_eq!(e.embedded, vec!["Inter".to_string()]);
        assert_eq!(e.fallback, vec!["Not Installed Anywhere".to_string()]);
        assert!(e.css.contains("font-family:'Inter'") && e.css.contains("font-weight:100 900") && e.css.contains("font-style:italic"));
        assert!(e.files.iter().any(|(p, b)| p.starts_with("fonts/Inter-") && p.ends_with(".ttf") && b.len() > 1000));
        assert!(e.files.iter().any(|(p, _)| p == "fonts/Inter-license.txt"));
    }

    #[test]
    fn a_css_stack_ends_in_a_generic_family_of_the_right_kind() {
        assert_eq!(css_stack("Lora"), "'Lora',serif");
        assert_eq!(css_stack("JetBrains Mono"), "'JetBrains Mono',monospace");
        assert_eq!(css_stack("Unknown One"), "'Unknown One',sans-serif");
    }

    #[test]
    fn a_downloaded_family_folder_is_read_back() {
        let dir = std::env::temp_dir().join(format!("bb-fonts-{}", std::process::id())).join("Fake Fira");
        std::fs::create_dir_all(&dir).unwrap();
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/Lato/Lato-Regular.ttf");
        std::fs::copy(&src, dir.join("Lato-Regular.ttf")).unwrap();
        std::fs::write(dir.join("meta.json"), r#"{"category":"Handwriting"}"#).unwrap();
        assert_eq!(load_family_dir(&dir).as_deref(), Some("Fake Fira"));
        let f = lookup("fake fira").unwrap();
        assert_eq!(f.source, Source::Downloaded);
        assert_eq!(f.category, Category::Handwriting);
        assert!(f.free());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }
}
