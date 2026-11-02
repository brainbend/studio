//! Passages for the fill-in, drag-the-words and mark-the-words questions: plain text where
//! `*answer*` marks a blank (or a word to mark) and `*a/b*` accepts either answer.

/// A piece of a passage.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Text(String),
    /// The accepted answers of one blank (never empty).
    Blank(Vec<String>),
}

/// Split `text` into plain text and blanks. A `*` with no closing `*` on the same line is plain text.
pub fn parse(text: &str) -> Vec<Part> {
    let mut out = vec![];
    let mut plain = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '*' {
            if let Some(len) = chars[i + 1..].iter().position(|&c| c == '*' || c == '\n') {
                if chars[i + 1 + len] == '*' {
                    let inner: String = chars[i + 1..i + 1 + len].iter().collect();
                    let answers: Vec<String> = inner.split('/').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
                    if !answers.is_empty() {
                        if !plain.is_empty() { out.push(Part::Text(std::mem::take(&mut plain))); }
                        out.push(Part::Blank(answers));
                        i += len + 2;
                        continue;
                    }
                }
            }
        }
        plain.push(chars[i]);
        i += 1;
    }
    if !plain.is_empty() { out.push(Part::Text(plain)); }
    out
}

/// The accepted answers of each blank, in order.
pub fn blanks(text: &str) -> Vec<Vec<String>> {
    parse(text).into_iter().filter_map(|p| if let Part::Blank(a) = p { Some(a) } else { None }).collect()
}

/// The passage with each blank shown as `____`, for places that cannot show a form.
pub fn with_gaps(text: &str) -> String {
    parse(text).into_iter().map(|p| match p { Part::Text(t) => t, Part::Blank(_) => "_____".to_string() }).collect()
}

/// The passage with the markers removed (answers in place).
pub fn answered(text: &str) -> String {
    parse(text).into_iter().map(|p| match p { Part::Text(t) => t, Part::Blank(a) => a[0].clone() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_alternatives_and_stray_stars() {
        let p = parse("The *cat/feline* sat on the *mat*. Not *closed");
        assert_eq!(p.len(), 5);
        assert_eq!(p[1], Part::Blank(vec!["cat".into(), "feline".into()]));
        assert_eq!(p[4], Part::Text(". Not *closed".into()));
        assert_eq!(blanks("a *b* c *d/e*"), vec![vec!["b".to_string()], vec!["d".to_string(), "e".to_string()]]);
        assert_eq!(with_gaps("Say *hi* now"), "Say _____ now");
        assert_eq!(answered("Say *hi/hello* now"), "Say hi now");
        assert!(blanks("no blanks here, just ** two stars").is_empty(), "an empty pair is not a blank");
        assert!(blanks("line *one\ntwo*").is_empty(), "a blank does not span lines");
    }
}
