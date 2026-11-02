//! English spell checking for a project's text, using the Hunspell en_US word list (SCOWL,
//! permissively licensed; see dict/LICENSE-en_US-SCOWL.txt) through the pure-Rust `spellbook`.
use std::collections::HashSet;
use std::sync::OnceLock;
use brainbend_doc::find::Place;
use brainbend_doc::Project;

static DICT: OnceLock<Option<spellbook::Dictionary>> = OnceLock::new();

fn dict() -> Option<&'static spellbook::Dictionary> {
    DICT.get_or_init(|| spellbook::Dictionary::new(include_str!("../dict/en_US.aff"), include_str!("../dict/en_US.dic")).ok()).as_ref()
}

/// Load the word list on a background thread so the first check does not stall the window.
pub fn warm() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| { std::thread::spawn(|| { dict(); }); });
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// Byte ranges of the words worth checking: not numbers, acronyms, camelCase words, one letters,
/// or parts of web and mail addresses.
pub fn words(text: &str) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        if !is_word_char(chars[i].1) { i += 1; continue; }
        let start = i;
        while i < chars.len() && (is_word_char(chars[i].1) || ((chars[i].1 == '\'' || chars[i].1 == '’') && i + 1 < chars.len() && is_word_char(chars[i + 1].1) && i > start)) { i += 1; }
        let (a, b) = (chars[start].0, chars.get(i).map(|c| c.0).unwrap_or(text.len()));
        let w = &text[a..b];
        // The whitespace-delimited chunk around it decides whether it is part of an address.
        let chunk_start = text[..a].rfind(char::is_whitespace).map(|p| p + 1).unwrap_or(0);
        let chunk_end = text[b..].find(char::is_whitespace).map(|p| b + p).unwrap_or(text.len());
        let chunk = &text[chunk_start..chunk_end];
        let address = chunk.contains("://") || chunk.contains('@') || chunk.to_ascii_lowercase().starts_with("www.");
        let letters: Vec<char> = w.chars().filter(|c| c.is_alphabetic()).collect();
        let skip = address
            || w.chars().any(|c| c.is_ascii_digit())
            || letters.len() < 2
            || letters.iter().all(|c| c.is_uppercase())
            || letters.iter().skip(1).any(|c| c.is_uppercase());
        if !skip { out.push((a, b)); }
    }
    out
}

/// Whether `word` is spelled correctly (or accepted by `custom`, which holds lowercase words).
pub fn is_correct(word: &str, custom: &HashSet<String>) -> bool {
    let w = word.replace('’', "'");
    let Some(d) = dict() else { return true };
    d.check(&w) || custom.contains(&w.to_lowercase())
}

/// Best-first suggestions for a misspelled word.
pub fn suggest(word: &str) -> Vec<String> {
    let mut out = vec![];
    if let Some(d) = dict() { d.suggest(&word.replace('’', "'"), &mut out); }
    out.truncate(6);
    out
}

/// One misspelled word in one place.
#[derive(Clone, Debug, PartialEq)]
pub struct Misspelling {
    pub slide: usize,
    pub place: Place,
    pub word: String,
    /// A short stretch of text around the first occurrence.
    pub context: String,
}

/// Every misspelled word in the project's text. `ignore` holds lowercase words to skip for now;
/// the project's own custom words are skipped too.
pub fn check_project(project: &Project, ignore: &HashSet<String>) -> Vec<Misspelling> {
    let mut custom: HashSet<String> = project.custom_words.iter().map(|w| w.to_lowercase()).collect();
    custom.extend(ignore.iter().cloned());
    let mut out = vec![];
    for (slide, place, text) in project.texts() {
        let mut seen = HashSet::new();
        for (a, b) in words(&text) {
            let w = &text[a..b];
            if is_correct(w, &custom) || !seen.insert(w.to_string()) { continue; }
            let chars: Vec<char> = text.chars().collect();
            let from_char = text[..a].chars().count();
            let (from, to) = (from_char.saturating_sub(18), (from_char + w.chars().count() + 18).min(chars.len()));
            let ctx: String = chars[from..to].iter().map(|c| if *c == '\n' { ' ' } else { *c }).collect();
            out.push(Misspelling { slide, place: place.clone(), word: w.to_string(), context: format!("{}{}{}", if from > 0 { "…" } else { "" }, ctx, if to < chars.len() { "…" } else { "" }) });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainbend_doc::*;

    #[test]
    fn tokenizes_and_skips_what_is_not_prose() {
        let t = "Teh quick brown fox's 2nd NASA iPhone visit https://exampel.com or joe@exampel.com. I am";
        let ws: Vec<&str> = words(t).into_iter().map(|(a, b)| &t[a..b]).collect();
        assert_eq!(ws, vec!["Teh", "quick", "brown", "fox's", "visit", "or", "am"]);
    }

    #[test]
    fn finds_misspellings_in_slides_and_honors_custom_words() {
        let mut p = Project::new("S");
        p.slides.push(Slide::new("s1", "Intro", 1920.0, 1080.0));
        p.sanitize();
        let mut o = Object::new("a", "A", Kind::Text, 0.0, 0.0, 100.0, 50.0);
        o.text = "Welcome to Brainbend. Safty first, safty always.".into();
        p.slides[0].layers[0].objects.push(o);
        p.slides[0].notes = "Remember the recieve step".into();
        let found = check_project(&p, &HashSet::new());
        let words: Vec<&str> = found.iter().map(|m| m.word.as_str()).collect();
        assert!(words.contains(&"Brainbend") && words.contains(&"Safty") && words.contains(&"safty") && words.contains(&"recieve"), "{words:?}");
        assert!(!words.contains(&"Welcome") && !words.contains(&"first"));
        assert!(found.iter().any(|m| m.place == Place::Notes && m.word == "recieve"));
        p.custom_words.push("brainbend".into());
        let mut ignore = HashSet::new();
        ignore.insert("recieve".to_string());
        let after: Vec<String> = check_project(&p, &ignore).into_iter().map(|m| m.word).collect();
        assert!(!after.contains(&"Brainbend".to_string()) && !after.contains(&"recieve".to_string()), "{after:?}");
        assert!(suggest("Safty").iter().any(|s| s == "Safety"), "{:?}", suggest("Safty"));
    }
}
