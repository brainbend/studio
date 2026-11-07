//! AI Tools: letting an AI tool the author chooses control BrainBend Studio, through MCP (see `brainbend-mcp`).
//!
//! It is off until the author turns it on in Help ▸ AI Tools, listens on this computer only, and answers only requests
//! that carry a secret token the author copies into their AI tool. What it may do is chosen there too: looking at the
//! project is always allowed once it is on; changing it can be switched off; opening, saving and publishing files is a
//! separate switch. Everything it does is listed in the window, and every change can be undone.
use super::*;
use brainbend_mcp as mcp;
use mcp_tools::Access;
use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The port the controller listens on unless the author's saved choice or a busy port says otherwise. It stays the same
/// from one run to the next so that the AI tool's setup keeps working.
const DEFAULT_PORT: u16 = 47631;
const LOG_LINES: usize = 60;

/// What the author chose, kept in `~/.brainbend-studio/mcp.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// The secret every request must carry. Made the first time it is needed and kept, so the AI tool's setup keeps working.
    pub token: String,
    pub port: u16,
    /// An AI tool may change the open project (every change can be undone).
    pub allow_edit: bool,
    /// An AI tool may open, save and publish files.
    pub allow_files: bool,
    /// Start listening when BrainBend Studio opens.
    pub on_launch: bool,
}

impl Default for Settings {
    fn default() -> Self { Self { token: String::new(), port: DEFAULT_PORT, allow_edit: true, allow_files: false, on_launch: false } }
}

impl Settings {
    fn path() -> Option<std::path::PathBuf> {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(std::path::PathBuf::from(home).join(".brainbend-studio").join("mcp.json"))
    }

    /// Reads saved settings; anything missing or unreadable takes its usual value (and a damaged file is simply replaced).
    pub fn parse(text: &str) -> Self {
        let d = Settings::default();
        let v: serde_json::Value = serde_json::from_str(text).unwrap_or(serde_json::Value::Null);
        Settings {
            token: v["token"].as_str().filter(|t| t.len() >= 32 && t.chars().all(|c| c.is_ascii_alphanumeric())).unwrap_or("").to_string(),
            port: v["port"].as_u64().and_then(|p| u16::try_from(p).ok()).unwrap_or(d.port),
            allow_edit: v["allow_edit"].as_bool().unwrap_or(d.allow_edit),
            allow_files: v["allow_files"].as_bool().unwrap_or(d.allow_files),
            on_launch: v["on_launch"].as_bool().unwrap_or(d.on_launch),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({ "token": self.token, "port": self.port, "allow_edit": self.allow_edit, "allow_files": self.allow_files, "on_launch": self.on_launch }).to_string()
    }

    pub fn load() -> Self { Self::path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Self::parse(&t)).unwrap_or_default() }

    /// Saves them. The token is a secret, so on a system with file permissions only the author may read the file.
    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        write_private(&path, &self.to_json());
    }
}

/// Writes a file that holds a secret (a token or an API key): replaced whole, and readable only by the author on a system
/// with file permissions. Failing to write it is not worth stopping for; the secret is just not kept.
pub fn write_private(path: &std::path::Path, text: &str) {
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    let tmp = path.with_extension("json.tmp");
    let written = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        options.open(&tmp)?.write_all(text.as_bytes())?;
        std::fs::rename(&tmp, path)
    })();
    if written.is_err() { let _ = std::fs::remove_file(&tmp); }
}

/// One line of the activity list.
pub struct LogLine {
    pub at: Instant,
    pub text: String,
    pub ok: bool,
}

#[derive(Default)]
pub struct Mcp {
    pub settings: Settings,
    /// Why it could not start, or a note about the port.
    pub error: Option<String>,
    pub log: VecDeque<LogLine>,
    pub last_call: Option<Instant>,
    /// Start on the next frame (the setting to start when Studio opens, which needs the window system to be up).
    start_wanted: bool,
    show_token: bool,
    handle: Option<mcp::Handle>,
    rx: Option<Receiver<mcp::ToolCall>>,
}

impl Mcp {
    /// The saved settings, for the app that is starting up.
    pub fn load() -> Self {
        let settings = Settings::load();
        let start_wanted = settings.on_launch;
        Self { settings, start_wanted, ..Self::default() }
    }

    pub fn running(&self) -> bool { self.handle.is_some() }

    pub fn url(&self) -> Option<String> { self.handle.as_ref().map(|h| h.url()) }

    pub fn stop(&mut self) {
        self.handle = None;
        self.rx = None;
    }

    /// Starts listening. A busy port falls back to one the system chooses, and says so.
    pub fn start(&mut self, ctx: &egui::Context) {
        self.stop();
        self.error = None;
        if self.settings.token.is_empty() {
            match mcp::new_token() {
                Ok(t) => { self.settings.token = t; self.settings.save(); }
                Err(e) => { self.error = Some(e); return; }
            }
        }
        let (tx, rx) = channel();
        let c = ctx.clone();
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || c.request_repaint());
        let config = |port| mcp::Config { token: self.settings.token.clone(), port, version: env!("CARGO_PKG_VERSION").into(), instructions: mcp_tools::INSTRUCTIONS.into() };
        let wanted = self.settings.port;
        let handle = mcp::start(config(wanted), mcp_tools::tools(), tx.clone(), wake.clone()).or_else(|first| {
            mcp::start(config(0), mcp_tools::tools(), tx, wake).inspect(|_| {
                self.error = Some(format!("Port {wanted} is in use ({first}), so a different one was chosen. The address below is the one to use."));
            })
        });
        match handle {
            Ok(h) => { self.handle = Some(h); self.rx = Some(rx); }
            Err(e) => self.error = Some(e),
        }
    }

    /// A new token. An AI tool that had the old one is refused until it is given the new one.
    pub fn new_token(&mut self, ctx: &egui::Context) {
        match mcp::new_token() {
            Ok(t) => { self.settings.token = t; self.settings.save(); if self.running() { self.start(ctx); } }
            Err(e) => self.error = Some(e),
        }
    }

    pub(crate) fn note(&mut self, tool: &str, result: &Result<String, String>) {
        self.last_call = Some(Instant::now());
        let (ok, text) = match result {
            Ok(_) => (true, tool.to_string()),
            Err(e) => (false, format!("{tool}: {}", e.chars().take(120).collect::<String>())),
        };
        self.log.push_front(LogLine { at: Instant::now(), text, ok });
        self.log.truncate(LOG_LINES);
    }
}

/// How long ago, in a few words.
fn ago(then: Instant) -> String {
    let s = then.elapsed().as_secs();
    if s < 2 { "just now".into() } else if s < 90 { format!("{s} s ago") } else if s < 5400 { format!("{} min ago", s / 60) } else { format!("{} h ago", s / 3600) }
}

impl App {
    /// Once a frame: start when asked, run the tool calls that have come in, and show the window.
    pub(super) fn mcp_frame(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.mcp.start_wanted) { self.mcp.start(ctx); }
        while let Some(call) = self.mcp.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            let result = self.mcp_dispatch(&call.name, &call.arguments);
            self.mcp.note(&call.name, &result);
            let _ = call.reply.send(result);
        }
    }

    /// Runs a tool if the author allows what it does.
    pub(crate) fn mcp_dispatch(&mut self, name: &str, args: &serde_json::Value) -> Result<String, String> {
        match mcp_tools::access(name) {
            Some(Access::Read) => {}
            Some(Access::Edit) if self.mcp.settings.allow_edit => {}
            Some(Access::Files) if self.mcp.settings.allow_files => {}
            Some(Access::Edit) => return Err("The author has not allowed AI tools to change the project. They can turn on \"Change the open project\" in BrainBend Studio, under Help > AI Tools.".into()),
            Some(Access::Files) => return Err("The author has not allowed AI tools to open, save or publish files. They can turn that on in BrainBend Studio, under Help > AI Tools.".into()),
            None => return Err(format!("There is no tool called {name}.")),
        }
        self.mcp_run(name, args)
    }

    /// The AI tab of the side panel: the chat with Claude, and below it everything about connecting an AI tool of your own.
    pub(super) fn ai_panel(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let mut start = false;
        let mut stop = false;
        let mut new_token = false;
        let mut copy: Option<String> = None;
        let mut changed = false;
        egui::ScrollArea::vertical().id_salt("ai-tab").auto_shrink([false, false]).show(ui, |ui| {
            self.chat_ui(ui);
            ui.add_space(6.0);
            ui.separator();
            let has_key = self.assistant.settings.key().is_some();
            egui::CollapsingHeader::new(egui::RichText::new("Connect your own AI tool").strong()).default_open(!has_key).show(ui, |ui| {
                ui.label("Use an AI tool you already have, such as Claude Code, instead of the chat above. It connects through MCP, runs only on this computer, and only while this is on.");
                ui.add_space(4.0);
                let mut on = self.mcp.running();
                if ui.checkbox(&mut on, "Let AI tools connect").changed() { if on { start = true; } else { stop = true; } }
                if let Some(e) = &self.mcp.error { ui.colored_label(egui::Color32::from_rgb(220, 70, 70), e); }
                if let Some(url) = self.mcp.url() {
                    ui.add_space(4.0);
                    let token = self.mcp.settings.token.clone();
                    ui.label(egui::RichText::new("Address").weak());
                    ui.horizontal_wrapped(|ui| {
                        ui.add(egui::Label::new(egui::RichText::new(&url).monospace()).selectable(true));
                        if ui.button("Copy").clicked() { copy = Some(url.clone()); }
                    });
                    ui.label(egui::RichText::new("Token").weak());
                    ui.add(egui::Label::new(egui::RichText::new(if self.mcp.show_token { token.clone() } else { "•".repeat(24) }).monospace()).selectable(self.mcp.show_token));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(if self.mcp.show_token { "Hide" } else { "Show" }).clicked() { self.mcp.show_token = !self.mcp.show_token; }
                        if ui.button("Copy token").clicked() { copy = Some(token.clone()); }
                        if ui.button("New token").on_hover_text("An AI tool that has the old token stops working until it is given the new one").clicked() { new_token = true; }
                    });
                    ui.add_space(4.0);
                    ui.label("To connect Claude Code, run this once in a terminal:");
                    let command = format!("claude mcp add --transport http brainbend {url} --header \"Authorization: Bearer {token}\"");
                    // Shown wrapped, so all of it can be read; the token stays hidden here until Show is pressed, but Copy takes the real one.
                    let shown = format!("claude mcp add --transport http brainbend {url} --header \"Authorization: Bearer {}\"", if self.mcp.show_token { token.as_str() } else { "<token>" });
                    ui.add(egui::Label::new(egui::RichText::new(shown).monospace()).wrap().selectable(true));
                    if ui.button("Copy command").on_hover_text("Copies the whole command, with the token").clicked() { copy = Some(command); }
                    ui.weak("Another AI tool: give it the address, and have it send the token as the header  Authorization: Bearer <token>.");
                }
            });
            egui::CollapsingHeader::new(egui::RichText::new("What AI may do").strong()).default_open(true).show(ui, |ui| {
                ui.weak("Looking at the lesson is always allowed. This covers the chat and any AI tool you connect.");
                changed |= ui.checkbox(&mut self.mcp.settings.allow_edit, "Change the open project").on_hover_text("Add, change and delete slides and objects. Undo takes it back.").changed();
                changed |= ui.add_enabled(false, egui::Checkbox::new(&mut self.mcp.settings.allow_files, "Open, save and publish files (coming soon)")).changed();
                changed |= ui.checkbox(&mut self.mcp.settings.on_launch, "Let AI tools connect when BrainBend Studio opens").changed();
            });
            egui::CollapsingHeader::new(egui::RichText::new("Activity").strong()).default_open(false).show(ui, |ui| {
                ui.weak(match self.mcp.last_call { Some(t) => format!("Last request {}", ago(t)), None => "No requests yet".into() });
                if self.mcp.log.is_empty() { ui.weak("What the chat and any AI tool do will be listed here."); }
                for l in self.mcp.log.iter().take(30) {
                    let text = format!("{}  {}", ago(l.at), l.text);
                    if l.ok { ui.label(text); } else { ui.colored_label(egui::Color32::from_rgb(220, 70, 70), text); }
                }
            });
            ui.add_space(6.0);
            ui.weak("An outside AI tool is answered only with the token, and web pages cannot reach it. Everything AI changes can be undone with Undo.");
        });
        if let Some(text) = copy { ctx.copy_text(text); }
        if changed { self.mcp.settings.save(); }
        if start { self.mcp.start(&ctx); }
        if stop { self.mcp.stop(); self.mcp.error = None; }
        if new_token { self.mcp.new_token(&ctx); }
        if !self.mcp.running() { self.mcp.show_token = false; }
        if self.mcp.running() || self.assistant.busy() { ctx.request_repaint_after(Duration::from_secs(1)); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_start_safe_and_survive_being_saved_and_read() {
        let d = Settings::default();
        assert_eq!((d.port, d.allow_edit, d.allow_files, d.on_launch), (DEFAULT_PORT, true, false, false), "changes are allowed, files and starting by itself are not");
        let s = Settings { token: "a".repeat(64), port: 5000, allow_edit: false, allow_files: true, on_launch: true };
        assert_eq!(Settings::parse(&s.to_json()), s);
    }

    #[test]
    fn a_damaged_or_odd_settings_file_never_leaves_the_controller_open_or_unlocked() {
        assert_eq!(Settings::parse("{broken"), Settings::default());
        assert_eq!(Settings::parse(""), Settings::default());
        let s = Settings::parse(r#"{"token":"short","port":99999,"allow_edit":"yes"}"#);
        assert_eq!((s.token.as_str(), s.port, s.allow_edit), ("", DEFAULT_PORT, true), "a weak token is discarded so a new one is made; odd values take their usual value");
        assert!(!Settings::parse(r#"{"token":"has spaces and symbols !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"}"#).token.contains(' '));
    }

    #[test]
    fn the_activity_list_keeps_the_newest_and_marks_failures() {
        let mut m = Mcp::default();
        m.note("list_slides", &Ok("[]".into()));
        m.note("add_text", &Err("The author is typing in a text box. Try again in a moment.".into()));
        assert_eq!(m.log.len(), 2);
        assert_eq!((m.log[0].ok, m.log[0].text.starts_with("add_text: The author")), (false, true), "newest first");
        assert!(m.log[1].ok && m.last_call.is_some());
        for i in 0..100 { m.note(&format!("t{i}"), &Ok(String::new())); }
        assert_eq!(m.log.len(), LOG_LINES);
        assert_eq!(m.log[0].text, "t99");
    }

    #[test]
    fn times_are_said_in_a_few_words() {
        let now = Instant::now();
        assert_eq!(ago(now), "just now");
        assert_eq!(ago(now - Duration::from_secs(30)), "30 s ago");
        assert_eq!(ago(now - Duration::from_secs(600)), "10 min ago");
        assert_eq!(ago(now - Duration::from_secs(7300)), "2 h ago");
    }
}
