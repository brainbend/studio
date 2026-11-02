//! Web Lessons: a scrolling lesson built from content blocks.
//!
//! A lesson is a list of sections (chapters), each an ordered list of blocks. Knowledge-check
//! blocks hold an ordinary `Question`, so every question type the slide player supports works
//! here too. Text blocks use a small Markdown subset (see `markdown_hint`).
use crate::check::{Issue, Location, Severity};
use crate::*;

/// When the lesson counts as complete.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Completion {
    /// Every block has been scrolled into view.
    AllViewed,
    /// Every knowledge check has been answered and the score reaches the pass mark.
    ChecksPassed,
    /// Both of the above.
    #[default]
    Both,
}

impl Completion {
    pub const ALL: [Completion; 3] = [Completion::Both, Completion::AllViewed, Completion::ChecksPassed];

    pub fn label(self) -> &'static str {
        match self {
            Completion::AllViewed => "All blocks viewed",
            Completion::ChecksPassed => "Knowledge checks passed",
            Completion::Both => "All blocks viewed and knowledge checks passed",
        }
    }
}

/// How an `Items` block lays out its entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemsStyle {
    #[default]
    Accordion,
    Tabs,
    /// Numbered steps.
    Process,
    Timeline,
    /// Cards that flip from the title (front) to the body (back).
    Flashcards,
}

impl ItemsStyle {
    pub const ALL: [ItemsStyle; 5] = [ItemsStyle::Accordion, ItemsStyle::Tabs, ItemsStyle::Process, ItemsStyle::Timeline, ItemsStyle::Flashcards];

    pub fn label(self) -> &'static str {
        match self {
            ItemsStyle::Accordion => "Accordion",
            ItemsStyle::Tabs => "Tabs",
            ItemsStyle::Process => "Process",
            ItemsStyle::Timeline => "Timeline",
            ItemsStyle::Flashcards => "Flashcards",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub title: String,
    pub body: String,
}

/// How wide an image block is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageWidth {
    #[default]
    Full,
    Medium,
    Small,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BlockKind {
    Heading { text: String, level: u8 },
    /// Paragraphs, bullets and numbers in the Markdown subset.
    Text { text: String },
    Image { asset: Option<String>, alt: String, caption: String, width: ImageWidth, decorative: bool, #[serde(default)] crop: Option<[f32; 4]> },
    /// A picture beside text; on narrow screens they stack.
    ImageText { asset: Option<String>, alt: String, text: String, image_right: bool, #[serde(default)] crop: Option<[f32; 4]> },
    Video { asset: Option<String>, captions: Option<String>, caption: String },
    Audio { asset: Option<String>, caption: String },
    /// A button that opens a web or mail address.
    Button { label: String, url: String },
    Divider,
    Spacer,
    Quote { text: String, by: String },
    /// A large callout sentence.
    Statement { text: String },
    Items { style: ItemsStyle, items: Vec<Item> },
    /// A graded or survey question (any style the slide player supports).
    KnowledgeCheck(Question),
    /// Hides everything below until the learner presses the button.
    Continue { label: String },
}

impl BlockKind {
    pub fn label(&self) -> &'static str {
        match self {
            BlockKind::Heading { .. } => "Heading",
            BlockKind::Text { .. } => "Text",
            BlockKind::Image { .. } => "Image",
            BlockKind::ImageText { .. } => "Image and text",
            BlockKind::Video { .. } => "Video",
            BlockKind::Audio { .. } => "Audio",
            BlockKind::Button { .. } => "Button",
            BlockKind::Divider => "Divider",
            BlockKind::Spacer => "Spacer",
            BlockKind::Quote { .. } => "Quote",
            BlockKind::Statement { .. } => "Statement",
            BlockKind::Items { style, .. } => style.label(),
            BlockKind::KnowledgeCheck(_) => "Knowledge check",
            BlockKind::Continue { .. } => "Continue",
        }
    }

    /// A fresh block of each kind a menu offers, in menu order, with a group heading for each run.
    pub fn catalog() -> Vec<(&'static str, Vec<BlockKind>)> {
        let items = |style, a: &str, b: &str| BlockKind::Items { style, items: vec![Item { title: a.into(), body: "Add the details here.".into() }, Item { title: b.into(), body: "Add the details here.".into() }] };
        vec![
            ("Text", vec![
                BlockKind::Heading { text: "Heading".into(), level: 2 },
                BlockKind::Text { text: "Write your text here. Use **bold**, *italic*, [links](https://example.com), and lines starting with - for bullets.".into() },
                BlockKind::Quote { text: "A quotation worth remembering.".into(), by: "Someone".into() },
                BlockKind::Statement { text: "The one thing to remember.".into() },
            ]),
            ("Media", vec![
                BlockKind::Image { asset: None, alt: String::new(), caption: String::new(), width: ImageWidth::Full, decorative: false, crop: None },
                BlockKind::ImageText { asset: None, alt: String::new(), text: "Describe the picture here.".into(), image_right: false, crop: None },
                BlockKind::Video { asset: None, captions: None, caption: String::new() },
                BlockKind::Audio { asset: None, caption: String::new() },
            ]),
            ("Interactive", vec![
                items(ItemsStyle::Accordion, "First topic", "Second topic"),
                items(ItemsStyle::Tabs, "Tab one", "Tab two"),
                items(ItemsStyle::Process, "Step one", "Step two"),
                items(ItemsStyle::Timeline, "2020", "2024"),
                items(ItemsStyle::Flashcards, "Front of card", "Another card"),
            ]),
            ("Knowledge check", vec![BlockKind::KnowledgeCheck(Question::new_single())]),
            ("Layout", vec![
                BlockKind::Button { label: "Learn more".into(), url: "https://example.com".into() },
                BlockKind::Continue { label: "Continue".into() },
                BlockKind::Divider,
                BlockKind::Spacer,
            ]),
        ]
    }

    /// Every piece of author-written text in the block (for find, replace and spelling).
    pub fn texts_mut(&mut self) -> Vec<&mut String> {
        match self {
            BlockKind::Heading { text, .. } | BlockKind::Text { text } | BlockKind::Statement { text } => vec![text],
            BlockKind::Image { alt, caption, .. } => vec![alt, caption],
            BlockKind::ImageText { alt, text, .. } => vec![alt, text],
            BlockKind::Video { caption, .. } | BlockKind::Audio { caption, .. } => vec![caption],
            BlockKind::Button { label, .. } | BlockKind::Continue { label } => vec![label],
            BlockKind::Quote { text, by } => vec![text, by],
            BlockKind::Items { items, .. } => items.iter_mut().flat_map(|i| [&mut i.title, &mut i.body]).collect(),
            BlockKind::KnowledgeCheck(q) => {
                let mut v: Vec<&mut String> = vec![&mut q.prompt, &mut q.feedback_correct, &mut q.feedback_incorrect];
                v.extend(q.choices.iter_mut().map(|c| &mut c.text));
                v
            }
            BlockKind::Divider | BlockKind::Spacer => vec![],
        }
    }

    pub fn texts(&self) -> Vec<String> {
        self.clone().texts_mut().into_iter().map(|t| t.clone()).collect()
    }

    /// The media this block uses.
    pub fn assets(&self) -> Vec<&str> {
        match self {
            BlockKind::Image { asset, .. } | BlockKind::ImageText { asset, .. } | BlockKind::Audio { asset, .. } => asset.as_deref().into_iter().collect(),
            BlockKind::Video { asset, captions, .. } => asset.as_deref().into_iter().chain(captions.as_deref()).collect(),
            BlockKind::KnowledgeCheck(q) => q.style.asset().into_iter().collect(),
            _ => vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub kind: BlockKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub title: String,
    /// One or two sentences shown on the module's card on the title page.
    #[serde(default)]
    pub summary: String,
    pub blocks: Vec<Block>,
}

/// The title banner at the top of the lesson.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cover {
    pub subtitle: String,
    /// A short paragraph under the title on the title page.
    #[serde(default)]
    pub description: String,
    pub image: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WebLesson {
    pub sections: Vec<Section>,
    #[serde(default)]
    pub completion: Completion,
    /// Show the section menu (a sidebar on wide screens, a drawer on phones).
    #[serde(default = "default_true")]
    pub show_menu: bool,
    #[serde(default)]
    pub cover: Option<Cover>,
    /// Overrides the theme's primary (buttons, progress, highlights) and secondary colours.
    #[serde(default)]
    pub primary: Option<Rgba>,
    #[serde(default)]
    pub secondary: Option<Rgba>,
}

impl WebLesson {
    /// A starter lesson: one section with a heading and a paragraph.
    pub fn starter() -> Self {
        WebLesson {
            sections: vec![Section {
                id: "section-1".into(),
                title: "Welcome".into(),
                summary: String::new(),
                blocks: vec![
                    Block { id: "block-1".into(), kind: BlockKind::Heading { text: "Why this matters".into(), level: 2 } },
                    Block { id: "block-2".into(), kind: BlockKind::Text { text: "Tell learners what this lesson covers and why it matters.".into() } },
                ],
            }],
            completion: Completion::default(),
            show_menu: true,
            cover: Some(Cover::default()),
            primary: None,
            secondary: None,
        }
    }

    pub fn blocks(&self) -> impl Iterator<Item = (usize, usize, &Block)> {
        self.sections.iter().enumerate().flat_map(|(si, s)| s.blocks.iter().enumerate().map(move |(bi, b)| (si, bi, b)))
    }

    pub fn question_count(&self) -> usize {
        self.blocks().filter(|(_, _, b)| matches!(b.kind, BlockKind::KnowledgeCheck(_))).count()
    }
}

impl Project {
    /// A new, empty Web Lesson project.
    pub fn new_web_lesson(name: &str, kind: ProjectKind) -> Self {
        let mut p = Project::new(name);
        p.kind = kind;
        p.lesson = Some(WebLesson::starter());
        p.sanitize();
        p
    }

    pub fn is_web(&self) -> bool {
        self.kind == ProjectKind::WebLesson && self.lesson.is_some()
    }

    /// Problems in a Web Lesson. `Location::slide` is the section and `object` is (block, 0).
    pub fn check_lesson(&self) -> Vec<Issue> {
        let mut out = vec![];
        let Some(l) = &self.lesson else { return out };
        let at = |s: usize, b: Option<usize>| Location { slide: s, object: b.map(|b| (b, 0)) };
        let issue = |sev, at, m: String| Issue { severity: sev, at, message: m };
        if l.sections.is_empty() {
            out.push(issue(Severity::Error, at(0, None), "The lesson has no sections.".into()));
        }
        for (si, s) in l.sections.iter().enumerate() {
            if s.title.trim().is_empty() { out.push(issue(Severity::Warning, at(si, None), format!("Section {} has no title, so the menu shows it blank.", si + 1))); }
            if s.blocks.is_empty() { out.push(issue(Severity::Warning, at(si, None), format!("Section {} has no blocks.", si + 1))); }
            for (bi, b) in s.blocks.iter().enumerate() {
                let here = |sev, m: &str| issue(sev, at(si, Some(bi)), format!("Section {}, block {} ({}): {m}", si + 1, bi + 1, b.kind.label()));
                match &b.kind {
                    BlockKind::Heading { text, .. } | BlockKind::Text { text } | BlockKind::Statement { text } | BlockKind::Quote { text, .. } if text.trim().is_empty() => out.push(here(Severity::Warning, "has no text.")),
                    BlockKind::Image { asset, alt, decorative, .. } => {
                        if !asset.as_ref().is_some_and(|a| self.assets.contains_key(a)) { out.push(here(Severity::Error, "has no picture.")); }
                        else if alt.trim().is_empty() && !decorative { out.push(here(Severity::Warning, "has no alt text (describe it, or mark it decorative).")); }
                    }
                    BlockKind::ImageText { asset, alt, .. } => {
                        if !asset.as_ref().is_some_and(|a| self.assets.contains_key(a)) { out.push(here(Severity::Error, "has no picture.")); }
                        else if alt.trim().is_empty() { out.push(here(Severity::Warning, "has no alt text.")); }
                    }
                    BlockKind::Video { asset, captions, .. } => {
                        if !asset.as_ref().is_some_and(|a| self.assets.contains_key(a)) { out.push(here(Severity::Error, "has no video.")); }
                        else if captions.is_none() { out.push(here(Severity::Warning, "has no captions.")); }
                    }
                    BlockKind::Audio { asset, .. } if !asset.as_ref().is_some_and(|a| self.assets.contains_key(a)) => out.push(here(Severity::Error, "has no audio clip.")),
                    BlockKind::Button { label, url } => {
                        if label.trim().is_empty() { out.push(here(Severity::Error, "has no label.")); }
                        if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")) { out.push(here(Severity::Error, "needs an address starting with https://, http:// or mailto:.")); }
                    }
                    BlockKind::Items { items, .. } => {
                        if items.is_empty() { out.push(here(Severity::Warning, "has no items.")); }
                        if items.iter().any(|i| i.title.trim().is_empty()) { out.push(here(Severity::Warning, "has an item with no title.")); }
                    }
                    BlockKind::KnowledgeCheck(q) => {
                        let mut qs = vec![];
                        self.check_question(q, &at(si, Some(bi)), &mut qs);
                        for mut i in qs { i.message = format!("Section {}, block {} (Knowledge check): {}", si + 1, bi + 1, i.message); out.push(i); }
                        if q.prompt.trim().is_empty() { /* already reported by check_question */ }
                    }
                    _ => {}
                }
            }
        }
        out
    }
}

/// A styled piece of a line of text.
#[derive(Clone, Debug, PartialEq)]
pub enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    /// Link text and address (only http, https and mailto addresses become links).
    Link(String, String),
}

/// One paragraph or list of a text block.
#[derive(Clone, Debug, PartialEq)]
pub enum MdBlock {
    Paragraph(Vec<Vec<Span>>),
    Bullets(Vec<Vec<Span>>),
    Numbers(Vec<Vec<Span>>),
}

fn is_safe_url(u: &str) -> bool {
    let l = u.to_ascii_lowercase();
    l.starts_with("https://") || l.starts_with("http://") || l.starts_with("mailto:")
}

/// Split one line into spans: `**bold**`, `*italic*`, `[text](url)`.
pub fn parse_inline(line: &str) -> Vec<Span> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<Span> = vec![];
    let mut buf = String::new();
    let mut i = 0;
    let flush = |buf: &mut String, out: &mut Vec<Span>| { if !buf.is_empty() { out.push(Span::Text(std::mem::take(buf))); } };
    while i < chars.len() {
        let rest = &chars[i..];
        let find = |from: usize, pat: &[char]| (from..chars.len().saturating_sub(pat.len() - 1)).find(|&k| chars[k..k + pat.len()] == *pat);
        if rest.starts_with(&['*', '*']) {
            if let Some(end) = find(i + 2, &['*', '*']).filter(|&e| e > i + 2) {
                flush(&mut buf, &mut out);
                out.push(Span::Bold(chars[i + 2..end].iter().collect()));
                i = end + 2;
                continue;
            }
        } else if rest[0] == '*' {
            if let Some(end) = find(i + 1, &['*']).filter(|&e| e > i + 1) {
                flush(&mut buf, &mut out);
                out.push(Span::Italic(chars[i + 1..end].iter().collect()));
                i = end + 1;
                continue;
            }
        } else if rest[0] == '[' {
            if let Some(close) = find(i + 1, &[']']).filter(|&c| chars.get(c + 1) == Some(&'(')) {
                if let Some(end) = find(close + 2, &[')']) {
                    let label: String = chars[i + 1..close].iter().collect();
                    let url: String = chars[close + 2..end].iter().collect();
                    if !url.chars().any(char::is_whitespace) {
                        flush(&mut buf, &mut out);
                        out.push(if is_safe_url(&url) { Span::Link(label, url) } else { Span::Text(label) });
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        buf.push(chars[i]);
        i += 1;
    }
    flush(&mut buf, &mut out);
    out
}

/// Parse a text block: blank lines separate paragraphs; lines starting `- ` or `* ` make bullets,
/// `1. ` numbers a list.
pub fn parse_markdown(text: &str) -> Vec<MdBlock> {
    let mut out = vec![];
    for para in text.split("\n\n") {
        let lines: Vec<&str> = para.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.is_empty() { continue; }
        let bullet = |l: &str| { let t = l.trim_start(); t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).map(str::to_string) };
        let number = |l: &str| { let t = l.trim_start(); let d = t.chars().take_while(|c| c.is_ascii_digit()).count(); (d > 0).then(|| t[d..].strip_prefix(". ").or_else(|| t[d..].strip_prefix(") "))).flatten().map(str::to_string) };
        if lines.iter().all(|l| bullet(l).is_some()) {
            out.push(MdBlock::Bullets(lines.iter().map(|l| parse_inline(&bullet(l).unwrap())).collect()));
        } else if lines.iter().all(|l| number(l).is_some()) {
            out.push(MdBlock::Numbers(lines.iter().map(|l| parse_inline(&number(l).unwrap())).collect()));
        } else {
            out.push(MdBlock::Paragraph(lines.iter().map(|l| parse_inline(l)).collect()));
        }
    }
    out
}

/// What the text editor tells authors about the text syntax.
pub fn markdown_hint() -> &'static str {
    "Blank line = new paragraph. **bold**, *italic*, [text](https://link). Lines starting with - are bullets; 1. numbers a list."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_web_lesson_has_a_section_and_survives_json() {
        let p = Project::new_web_lesson("L", ProjectKind::WebLesson);
        assert!(p.is_web());
        assert_eq!(p.lesson.as_ref().unwrap().sections[0].blocks.len(), 2);
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(back.lesson, p.lesson);
        // Older files without a lesson still load.
        assert!(Project::new("old").lesson.is_none());
    }

    #[test]
    fn every_catalog_block_has_a_label_and_the_check_flags_the_empty_ones() {
        let mut p = Project::new_web_lesson("L", ProjectKind::WebLesson);
        let blocks: Vec<Block> = BlockKind::catalog().into_iter().flat_map(|(_, v)| v).enumerate().map(|(i, k)| Block { id: format!("b{i}"), kind: k }).collect();
        assert!(blocks.iter().all(|b| !b.kind.label().is_empty()));
        p.lesson.as_mut().unwrap().sections[0].blocks = blocks;
        let msgs: Vec<String> = p.check_lesson().into_iter().map(|i| i.message).collect();
        assert!(msgs.iter().any(|m| m.contains("(Image): has no picture")), "{msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("(Video): has no video")), "{msgs:?}");
        assert!(!msgs.iter().any(|m| m.contains("Button")), "the default button has a valid address: {msgs:?}");
    }

    #[test]
    fn markdown_parses_paragraphs_lists_and_inline_styles() {
        let md = parse_markdown("Read **this** and *that* [here](https://x.com) [bad](javascript:alert(1))\n\n- one\n- two\n\n1. first\n2. second");
        assert_eq!(md.len(), 3);
        let MdBlock::Paragraph(lines) = &md[0] else { panic!() };
        assert_eq!(lines[0][1], Span::Bold("this".into()));
        assert_eq!(lines[0][3], Span::Italic("that".into()));
        assert_eq!(lines[0][5], Span::Link("here".into(), "https://x.com".into()));
        assert!(matches!(&lines[0][7], Span::Text(t) if t == "bad"), "unsafe links lose their address: {:?}", lines[0]);
        assert!(matches!(&md[1], MdBlock::Bullets(b) if b.len() == 2));
        assert!(matches!(&md[2], MdBlock::Numbers(n) if n.len() == 2));
    }

    #[test]
    fn block_text_is_reachable_for_search() {
        let mut k = BlockKind::Items { style: ItemsStyle::Tabs, items: vec![Item { title: "A".into(), body: "b".into() }] };
        for t in k.texts_mut() { t.push('!'); }
        assert_eq!(k.texts(), vec!["A!".to_string(), "b!".to_string()]);
    }
}
