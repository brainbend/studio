//! Lists the font files under assets/fonts/<Family>/ so they can be compiled into the app.
use std::{fmt::Write, fs, path::Path};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut out = String::from("pub static BUNDLED: &[(&str, &[(&str, &[u8])], &str)] = &[\n");
    let mut families: Vec<_> = fs::read_dir(&root).unwrap().filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).collect();
    families.sort_by_key(|e| e.file_name());
    for fam in families {
        let name = fam.file_name().to_string_lossy().to_string();
        let mut files: Vec<_> = fs::read_dir(fam.path()).unwrap().filter_map(|e| e.ok())
            .filter(|e| matches!(e.path().extension().and_then(|x| x.to_str()), Some("ttf" | "otf"))).collect();
        files.sort_by_key(|e| e.file_name());
        if files.is_empty() { continue; }
        let _ = write!(out, "    ({name:?}, &[");
        for f in &files {
            let p = f.path().canonicalize().unwrap();
            let _ = write!(out, "({:?}, include_bytes!({:?}) as &[u8]), ", f.file_name().to_string_lossy(), p.to_string_lossy());
        }
        let lic = fam.path().join("OFL.txt");
        if lic.exists() {
            let _ = writeln!(out, "], include_str!({:?})),", lic.canonicalize().unwrap().to_string_lossy());
        } else {
            let _ = writeln!(out, "], \"\"),");
        }
    }
    out.push_str("];\n");
    fs::write(Path::new(&std::env::var("OUT_DIR").unwrap()).join("bundled.rs"), out).unwrap();
}
