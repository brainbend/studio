// Tally the import notes across .story files: notes <file.story>...
use std::collections::BTreeMap;
fn main() {
    let mut tally: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for p in std::env::args().skip(1) {
        let file = std::path::Path::new(&p).file_name().unwrap().to_string_lossy().to_string();
        match brainbend_import_legacy::import(std::fs::File::open(&p).unwrap(), "x") {
            Err(e) => println!("{file}: IMPORT FAILED: {e}"),
            Ok(proj) => for n in &proj.notes {
                let msg = n.split_once(": ").map(|x| x.1).unwrap_or(n);
                let msg = msg.rsplit_once(" (x").filter(|x| x.1.ends_with(')')).map(|x| x.0).unwrap_or(msg).to_string();
                let c = n.rsplit_once(" (x").and_then(|x| x.1.trim_end_matches(')').parse::<usize>().ok()).unwrap_or(1);
                let e = tally.entry(msg).or_default();
                e.0 += c;
                if !e.1.contains(&file) { e.1.push(file.clone()); }
            },
        }
    }
    let mut v: Vec<_> = tally.into_iter().collect();
    v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    for (m, (c, files)) in v { println!("{c:>5}  {m}\n         in {}", files.join(", ")); }
}
