//! The project's title and the player's settings, for the AI tools: what is set now, and changing it.
//!
//! The same rule as for the design applies. Settings the author has not touched can be set freely; one the author
//! has already chosen is not changed until the author has agreed (`approved`). Everything is one undo step.

use super::*;
use crate::mcp_tools::*;
use serde_json::{json, Value};

fn untitled(name: &str) -> bool { let t = name.trim(); t.is_empty() || t.eq_ignore_ascii_case("untitled") }

/// The settings this tool changes, by the name it uses for them.
#[derive(Clone, Debug, PartialEq)]
struct Settings {
    title: String,
    show_title: bool,
    show_nav: bool,
    show_progress: bool,
    show_menu: bool,
    menu_free: bool,
    sidebar: &'static str,
    player_theme: &'static str,
    accent: Rgba,
    notes_tab: bool,
    glossary_tab: bool,
    search: bool,
    controls: [bool; 7],
}

const CONTROLS: [&str; 7] = ["play_pause", "seekbar", "replay", "volume", "captions", "speed", "fullscreen"];

impl Settings {
    fn of(p: &Project) -> Self {
        let c = p.player.controls;
        Self {
            title: p.name.clone(), show_title: p.player.show_title, show_nav: p.show_nav, show_progress: p.show_progress, show_menu: p.show_menu, menu_free: p.menu_free,
            sidebar: match p.player.sidebar { SidebarSide::Off => "off", SidebarSide::Left => "left", SidebarSide::Right => "right" },
            player_theme: match p.player.theme { PlayerTheme::Light => "light", _ => "dark" },
            accent: p.player.accent, notes_tab: p.player.tab_notes, glossary_tab: p.player.tab_glossary, search: p.player.search,
            controls: [c.play_pause, c.seekbar, c.replay, c.volume, c.captions, c.speed, c.fullscreen],
        }
    }

    fn apply(&self, p: &mut Project) {
        p.name = self.title.clone();
        p.player.show_title = self.show_title;
        p.show_nav = self.show_nav;
        p.show_progress = self.show_progress;
        p.show_menu = self.show_menu;
        p.menu_free = self.menu_free;
        p.player.sidebar = match self.sidebar { "left" => SidebarSide::Left, "right" => SidebarSide::Right, _ => SidebarSide::Off };
        p.player.theme = if self.player_theme == "light" { PlayerTheme::Light } else { PlayerTheme::Dark };
        p.player.accent = self.accent;
        p.player.tab_notes = self.notes_tab;
        p.player.tab_glossary = self.glossary_tab;
        p.player.search = self.search;
        let c = &mut p.player.controls;
        [c.play_pause, c.seekbar, c.replay, c.volume, c.captions, c.speed, c.fullscreen] = self.controls;
    }

    fn json(&self) -> Value {
        let mut v = json!({
            "title": self.title, "title_is_missing": untitled(&self.title), "show_title_in_player": self.show_title,
            "previous_next_buttons": self.show_nav, "progress_bar": self.show_progress, "slide_menu": self.show_menu, "learners_can_skip_from_menu": self.menu_free,
            "sidebar": self.sidebar, "player_theme": self.player_theme, "accent": self.accent.hex(),
            "notes_tab": self.notes_tab, "glossary_tab": self.glossary_tab, "search": self.search,
        });
        v["controls"] = CONTROLS.iter().zip(self.controls).map(|(n, on)| (n.to_string(), json!(on))).collect::<serde_json::Map<_, _>>().into();
        v
    }
}

impl App {
    /// The title and player settings as they are now, for `get_project`.
    pub(super) fn player_summary(&self) -> Value {
        self.project.as_ref().map(|p| Settings::of(p).json()).unwrap_or(Value::Null)
    }

    fn settings_print(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let Some(p) = self.project.as_ref() else { return 0 };
        let s = Settings::of(p);
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{s:?}").hash(&mut h);
        h.finish()
    }

    /// Changes the title and the player's settings. Give only what you want to change.
    pub(super) fn mcp_set_player(&mut self, a: &Value) -> Result<String, String> {
        let p = self.project.as_ref().unwrap();
        let now = Settings::of(p);
        let stock = Settings::of(&Project::new("Untitled"));
        let mut next = now.clone();
        let mut asked: Vec<String> = vec![];
        // What the author has already chosen differently from the starting settings: each is told in words when it would be overridden.
        let mut in_the_way: Vec<String> = vec![];
        macro_rules! flag { ($key:literal, $field:ident, $what:expr) => {
            if let Some(v) = bool_arg(a, $key)? {
                if v != now.$field { if now.$field != stock.$field { in_the_way.push(format!("{} is {}", $what, if now.$field { "on" } else { "off" })); } next.$field = v; asked.push($key.into()); }
            }
        } }
        if let Some(t) = text_arg(a, "title")?.map(|t| t.trim().to_string()) {
            if t.is_empty() { return Err("`title` must be some text.".into()); }
            if t != now.title { if !untitled(&now.title) { in_the_way.push(format!("the title is {:?}", now.title)); } next.title = t; asked.push("title".into()); }
        }
        flag!("show_title", show_title, "showing the title in the player");
        flag!("show_nav", show_nav, "the Previous / Next buttons");
        flag!("show_progress", show_progress, "the progress bar");
        flag!("show_menu", show_menu, "the slide menu");
        flag!("menu_free", menu_free, "skipping from the menu");
        flag!("notes_tab", notes_tab, "the Notes tab");
        flag!("glossary_tab", glossary_tab, "the Glossary tab");
        flag!("search", search, "the slide search");
        if let Some(s) = text_arg(a, "sidebar")? {
            let s: &'static str = match s.to_ascii_lowercase().as_str() { "off" | "none" => "off", "left" => "left", "right" => "right", _ => return Err("`sidebar` must be off, left or right.".into()) };
            if s != now.sidebar { if now.sidebar != stock.sidebar { in_the_way.push(format!("the sidebar is {}", now.sidebar)); } next.sidebar = s; asked.push("sidebar".into()); }
        }
        if let Some(s) = text_arg(a, "player_theme")? {
            let s: &'static str = match s.to_ascii_lowercase().as_str() { "dark" => "dark", "light" => "light", _ => return Err("`player_theme` must be dark or light.".into()) };
            if s != now.player_theme { if now.player_theme != stock.player_theme { in_the_way.push(format!("the player theme is {}", now.player_theme)); } next.player_theme = s; asked.push("player_theme".into()); }
        }
        if let Some(c) = color_arg(a, "accent")? {
            if c != now.accent { if now.accent != stock.accent { in_the_way.push(format!("the accent color is {}", now.accent.hex())); } next.accent = c; asked.push("accent".into()); }
        }
        if let Some(c) = a.get("controls").filter(|v| !v.is_null()) {
            let c = c.as_object().ok_or("`controls` must be an object such as {\"fullscreen\": true}.")?;
            for (k, v) in c {
                let i = CONTROLS.iter().position(|n| n == k).ok_or_else(|| format!("`controls` has no control called {k:?}. They are: {}.", CONTROLS.join(", ")))?;
                let on = v.as_bool().ok_or_else(|| format!("`controls`: {k} must be true or false."))?;
                if on != now.controls[i] { if now.controls[i] != stock.controls[i] { in_the_way.push(format!("the {k} control is {}", if now.controls[i] { "on" } else { "off" })); } next.controls[i] = on; asked.push(format!("controls.{k}")); }
            }
        }
        if a.as_object().is_none_or(|o| !o.keys().any(|k| k != "approved")) { return Err("Give at least one of title, show_title, show_nav, show_progress, show_menu, menu_free, sidebar, player_theme, accent, notes_tab, glossary_tab, search or controls.".into()); }
        if !in_the_way.is_empty() && self.mcp.settings_seen != Some(self.settings_print()) && !bool_arg(a, "approved")?.unwrap_or(false) {
            return Err(format!(
                "Not done: the author has already chosen: {}. Do not change those without asking. Tell the author what you would change and why, and ask whether to go ahead; \
if they say yes, call this again with approved set to true. Settings the author has not touched can be set without asking. Nothing was changed.",
                in_the_way.join("; ")
            ));
        }
        if asked.is_empty() { return done(json!({ "changed": [], "note": "Everything asked for was already set.", "settings": now.json() })); }
        self.snapshot();
        next.apply(self.project.as_mut().unwrap());
        self.dirty = true;
        self.status = "AI tool: set the title and player settings".into();
        self.mcp.settings_seen = Some(self.settings_print());
        let p = self.project.as_ref().unwrap();
        // Without Previous / Next the learner needs some way to move, if there is more than one slide.
        let own_navigation = p.slides.iter().any(|s| s.triggers.iter().any(|t| matches!(t.action, Action::JumpToSlide(_) | Action::NextSlide | Action::PreviousSlide)));
        let mut warnings: Vec<&str> = vec![];
        if !p.show_nav && p.slides.len() > 1 && !p.show_menu && !own_navigation {
            warnings.push("The Previous / Next buttons are off and the lesson has no navigation of its own (no slide menu, and no button or trigger that goes to another slide), so learners cannot move between slides. Add buttons with a go-to-slide trigger, or turn the slide menu on.");
        }
        done(json!({ "changed": asked, "settings": Settings::of(p).json(), "warnings": warnings }))
    }
}
