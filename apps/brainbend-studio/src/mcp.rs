//! AI Tools: letting an AI tool the author chooses control BrainBend Studio, through MCP (see `brainbend-mcp`).
//!
//! It is off until the author turns it on in Help ▸ Set up AI, listens on this computer only, and answers only requests
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

    /// Whether anything has connected: being on only means this app is listening.
    pub fn status_line(&self) -> String {
        match self.last_call { Some(t) => format!("• Connected. Last request {}.", ago(t)), None => "• On, and waiting for an AI tool to connect.".to_string() }
    }

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
        let config = |port| mcp::Config { token: self.settings.token.clone(), port, version: env!("CARGO_PKG_VERSION").into(), instructions: mcp_tools::INSTRUCTIONS.into(), pickup_ms: mcp::PICKUP_MS };
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
            // A call the AI tool was already told had failed (this window was not being drawn) is not run.
            if !call.claim() { continue; }
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
            Some(Access::Edit) => return Err("The author has not allowed AI tools to change the project. They can turn on \"Change the open project\" in BrainBend Studio, under Help > Set up AI.".into()),
            Some(Access::Files) => return Err("The author has not allowed AI tools to open, save or publish files. They can turn that on in BrainBend Studio, under Help > Set up AI.".into()),
            None => return Err(format!("There is no tool called {name}.")),
        }
        self.mcp_run(name, args)
    }

    /// The AI tab of the side panel: one button to set AI up, and once it is set up, the chat or the terminal.
    pub(super) fn ai_panel(&mut self, ui: &mut egui::Ui) {
        let has_key = self.assistant.settings.key().is_some();
        let connected = self.mcp.running();
        let has_terminal = self.assistant.terminal.is_some();
        if !has_key && !connected && !has_terminal {
            ui.add_space(14.0);
            ui.label("Ask an AI to build and change your lesson while you watch.");
            ui.add_space(8.0);
            if ui.add(egui::Button::new(egui::RichText::new("Set up AI").strong()).min_size(egui::vec2(120.0, 30.0))).clicked() { self.assistant.setup_open = true; }
            return;
        }
        if has_key && has_terminal {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.assistant.view_terminal, false, "Chat");
                ui.selectable_value(&mut self.assistant.view_terminal, true, "Terminal");
            });
        }
        if self.assistant.shows_terminal() {
            self.terminal_ui(ui);
        } else if has_key {
            self.chat_ui(ui);
            if connected { ui.weak(self.mcp.status_line()); }
        } else {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(self.mcp.status_line()).strong());
            ui.weak("What it does is listed here. Undo takes it back.");
            egui::ScrollArea::vertical().id_salt("ai-activity").auto_shrink([false, true]).max_height((ui.available_height() - 50.0).max(100.0)).show(ui, |ui| self.activity_ui(ui));
            if ui.small_button("Settings").clicked() { self.assistant.setup_open = true; }
        }
        if connected || self.assistant.busy() { ui.ctx().request_repaint_after(Duration::from_secs(1)); }
    }

    /// What the chat and any connected AI tool have done lately.
    fn activity_ui(&self, ui: &mut egui::Ui) {
        ui.weak(match self.mcp.last_call { Some(t) => format!("Last request {}", ago(t)), None => "No requests yet".into() });
        for l in self.mcp.log.iter().take(30) {
            let text = format!("{}  {}", ago(l.at), l.text);
            if l.ok { ui.label(text); } else { ui.colored_label(egui::Color32::from_rgb(220, 70, 70), text); }
        }
    }

    /// The Set up AI window: an API key, or a subscription's own tool; and what AI may do.
    pub(super) fn ai_setup_window(&mut self, ctx: &egui::Context) {
        if !self.assistant.setup_open { return; }
        let mut open = true;
        egui::Window::new("Set up AI").open(&mut open).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).default_width(400.0).show(ctx, |ui| {
            ui.set_max_width(400.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.assistant.setup_mode, 0, "Use an API key");
                ui.selectable_value(&mut self.assistant.setup_mode, 1, "Use a subscription");
            });
            ui.separator();
            if self.assistant.setup_mode == 0 { self.setup_key_ui(ui); } else { self.setup_connect_ui(ui, ctx); }
            ui.add_space(6.0);
            let mut changed = false;
            egui::CollapsingHeader::new("What AI may do").show(ui, |ui| {
                changed |= ui.checkbox(&mut self.mcp.settings.allow_edit, "Change the open project").on_hover_text("Add, change and delete slides and objects. Undo takes it back.").changed();
                changed |= ui.add_enabled(false, egui::Checkbox::new(&mut self.mcp.settings.allow_files, "Open, save and publish files (coming soon)")).changed();
                changed |= ui.checkbox(&mut self.mcp.settings.on_launch, "Let AI tools connect when BrainBend Studio opens").changed();
            });
            if changed { self.mcp.settings.save(); }
            egui::CollapsingHeader::new("Activity").show(ui, |ui| {
                if self.mcp.log.is_empty() { ui.weak("Nothing yet."); }
                self.activity_ui(ui);
            });
            ui.add_space(6.0);
            if ui.button("Done").clicked() { self.assistant.setup_open = false; }
        });
        if !open { self.assistant.setup_open = false; }
        if self.mcp.running() || self.assistant.busy() { ctx.request_repaint_after(Duration::from_secs(1)); }
        if !self.mcp.running() { self.mcp.show_token = false; }
    }

    /// Setting up with an API key: who, which model, and the key.
    fn setup_key_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Provider");
            self.provider_picker(ui);
        });
        let provider = self.assistant.settings.provider;
        let idx = provider.index();
        let saved = !self.assistant.settings.keys[idx].trim().is_empty();
        if self.assistant.settings.key().is_some() {
            ui.horizontal_wrapped(|ui| {
                ui.label(if saved { "✓ Key saved on this computer" } else { "✓ Using the key in your environment" });
                if saved && ui.small_button("Remove key").clicked() && !self.assistant.busy() { self.assistant.settings.keys[idx].clear(); self.assistant.settings.save(); }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Model");
                self.model_picker(ui);
            });
            self.other_model_ui(ui);
            ui.add_space(4.0);
            ui.weak("Done. The chat is in the AI tab.");
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.assistant.key_input).password(true).hint_text(provider.key_hint()).desired_width(200.0));
                let ready = !self.assistant.key_input.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new("Save key")).clicked() {
                    self.assistant.settings.keys[idx] = self.assistant.key_input.trim().to_string();
                    self.assistant.key_input.clear();
                    self.assistant.settings.save();
                }
            });
            if ui.link(format!("Get a key at {}", provider.billing_site())).clicked() { update::open_in_browser(provider.key_url()); }
        }
        ui.add_space(4.0);
        ui.weak(format!("Your lesson's content that {} reads goes to {} and is billed to your key.", provider.name(), provider.company()));
    }

    /// Setting up with a subscription: it cannot sign in here, but the company's own tool can connect.
    fn setup_connect_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.weak("A subscription works in the company's own tool, not through this app. Run that tool in the AI tab, already connected to your lesson.");
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("terminal-tool").selected_text(crate::terminal::TOOLS[self.assistant.term_tool.min(3)].name).show_ui(ui, |ui| {
                for (i, t) in crate::terminal::TOOLS.iter().enumerate() { ui.selectable_value(&mut self.assistant.term_tool, i, t.name); }
            });
            if ui.button("Open in the AI tab").clicked() {
                let tool = self.assistant.term_tool;
                self.start_terminal(tool, ctx);
                if self.assistant.terminal.is_some() { self.assistant.setup_open = false; self.right_tab = crate::AI_TAB; }
            }
        });
        ui.weak(crate::terminal::TOOLS[self.assistant.term_tool.min(3)].note);
        if let Some(e) = &self.assistant.terminal_error { ui.colored_label(egui::Color32::from_rgb(220, 70, 70), e); }
        ui.add_space(4.0);
        egui::CollapsingHeader::new("From your own terminal").show(ui, |ui| self.setup_own_terminal_ui(ui, ctx));
    }

    /// Connecting a tool that runs in the author's own terminal window.
    fn setup_own_terminal_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mut start = false;
        let mut stop = false;
        let mut new_token = false;
        let mut copy: Option<String> = None;
        let mut on = self.mcp.running();
        if ui.checkbox(&mut on, "Let AI tools connect").changed() { if on { start = true; } else { stop = true; } }
        if let Some(e) = &self.mcp.error { ui.colored_label(egui::Color32::from_rgb(220, 70, 70), e); }
        if let Some(url) = self.mcp.url() {
            let token = self.mcp.settings.token.clone();
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label("Connect:");
                for (i, tool) in CONNECT_TOOLS.iter().enumerate() { ui.selectable_value(&mut self.assistant.connect_tool, i, tool.name); }
            });
            let tool = &CONNECT_TOOLS[self.assistant.connect_tool.min(CONNECT_TOOLS.len() - 1)];
            ui.label(format!("Run {} in a terminal:", if tool.steps > 1 { "these" } else { "this" }));
            // Shown wrapped, so all of it can be read; the token stays hidden here until Show is pressed, but Copy takes the real one.
            let shown = connect_command(tool, &url, if self.mcp.show_token { token.as_str() } else { "<token>" });
            ui.add(egui::Label::new(egui::RichText::new(shown).monospace()).wrap().selectable(true));
            if ui.button("Copy command").on_hover_text("Copies the whole command, with the token").clicked() { copy = Some(connect_command(tool, &url, &token)); }
            egui::CollapsingHeader::new("Address and token").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Label::new(egui::RichText::new(&url).monospace()).selectable(true));
                    if ui.button("Copy").clicked() { copy = Some(url.clone()); }
                });
                ui.add(egui::Label::new(egui::RichText::new(if self.mcp.show_token { token.clone() } else { "•".repeat(24) }).monospace()).selectable(self.mcp.show_token));
                ui.horizontal_wrapped(|ui| {
                    if ui.button(if self.mcp.show_token { "Hide" } else { "Show" }).clicked() { self.mcp.show_token = !self.mcp.show_token; }
                    if ui.button("Copy token").clicked() { copy = Some(token.clone()); }
                    if ui.button("New token").on_hover_text("A tool that has the old token stops working until it is given the new one").clicked() { new_token = true; }
                });
                ui.weak("Another tool: give it the address, and have it send the token as the header  Authorization: Bearer <token>.");
            });
            ui.label(egui::RichText::new(self.mcp.status_line()).strong());
            if self.mcp.last_call.is_none() {
                ui.weak("Run the command, then start the tool in any folder and ask it about your lesson. Keep this window in view, for example beside the terminal: it cannot act while hidden or covered.");
            }
            ui.weak("Only this computer can reach it, and only with the token.");
        }
        if let Some(text) = copy { ctx.copy_text(text); }
        if start { self.mcp.start(ctx); }
        if stop { self.mcp.stop(); self.mcp.error = None; }
        if new_token { self.mcp.new_token(ctx); }
    }
}

/// An outside AI tool that can be connected, and how to tell it about this app.
pub(super) struct ConnectTool {
    pub name: &'static str,
    /// How many lines the command has.
    pub steps: usize,
    /// The command, with `{url}` and `{token}` to fill in.
    pub command: &'static str,
}

/// Claude Code, Codex and Gemini CLI. ChatGPT and Gemini in the browser cannot reach a program on this computer, but these
/// command-line tools can, and they use the author's own subscription or key. Each line is that tool's own way of adding an
/// MCP server that is reached over HTTP with a bearer token.
pub(super) const CONNECT_TOOLS: [ConnectTool; 3] = [
    ConnectTool { name: "Claude Code", steps: 1, command: "claude mcp add --scope user --transport http brainbend {url} --header \"Authorization: Bearer {token}\"" },
    ConnectTool { name: "Codex", steps: 2, command: "export BRAINBEND_TOKEN=\"{token}\"\ncodex mcp add brainbend --url {url} --bearer-token-env-var BRAINBEND_TOKEN" },
    ConnectTool { name: "Gemini CLI", steps: 1, command: "gemini mcp add --scope user --transport http --header \"Authorization: Bearer {token}\" brainbend {url}" },
];

/// The command for a tool, with this app's address and the token (or a stand-in for it).
pub(super) fn connect_command(tool: &ConnectTool, url: &str, token: &str) -> String { tool.command.replace("{url}", url).replace("{token}", token) }

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

    #[test]
    fn each_tool_gets_a_command_with_the_address_and_the_token() {
        for tool in &CONNECT_TOOLS {
            let c = connect_command(tool, "http://127.0.0.1:5000/mcp", "TOKEN123");
            assert!(c.contains("http://127.0.0.1:5000/mcp") && c.contains("TOKEN123") && !c.contains('{'), "{}: {c}", tool.name);
            assert!(c.contains("brainbend") && c.lines().count() == tool.steps, "{}: {c}", tool.name);
        }
        assert!(connect_command(&CONNECT_TOOLS[0], "u", "t").starts_with("claude mcp add --scope user --transport http"));
        let codex = connect_command(&CONNECT_TOOLS[1], "u", "t");
        assert!(codex.starts_with("export BRAINBEND_TOKEN=") && codex.contains("codex mcp add") && codex.lines().count() == 2);
        assert!(connect_command(&CONNECT_TOOLS[2], "u", "t").starts_with("gemini mcp add --scope user"));
    }
}
