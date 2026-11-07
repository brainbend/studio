//! The chat in the AI tab: the author asks, Claude builds and changes the lesson with the same tools an outside AI tool gets.
//!
//! Claude is reached with the author's own API key (kept on this computer, readable only by them); what Claude reads of the
//! lesson and what the author types go to Anthropic and are billed to that key, which the tab says. The conversation runs on
//! a background thread (`brainbend-assistant`); each tool Claude uses is handed back to the app's own thread, through the
//! same permission switches and undo steps as an outside AI tool, so it shows on screen and can be undone.
use super::*;
use brainbend_assistant as ai;
use brainbend_mcp::ToolCall;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// What Claude is told about its job. The tools tell it the rest.
const SYSTEM: &str = "You are the assistant built into BrainBend Studio, an app for building interactive lessons (slides with objects, \
animations, timelines, triggers, questions). The author talks to you in a chat beside their slides and sees the slides change as you work. \
You change the open lesson with your tools.\n\
- Look before you change: get_project and list_slides, then get_slide for the slide you will work on. Use describe_model before setting \
anything you are unsure of (a type name such as Object, Entrance, Emphasis, Trigger, Action, TextStyle), rather than guessing field names.\n\
- Work in small steps and check what a change did if it matters. Every change is one undo step, and the author can undo any of them.\n\
- Ask a short question when a request could mean very different things; otherwise make a sensible choice and say what you chose.\n\
- Make lessons that read well: short text, large enough to read (titles about 60 to 80 pixels and body text 36 or more on a 1280 by 720 slide), strong \
contrast between text and its background, objects kept inside the slide and not overlapping by accident, and a name for each object that says what it is.\n\
- Do not invent facts the author did not give you. Do not delete things the author did not ask you to.\n\
- When you have finished, say in one to three sentences what you did.\n\n";

/// The settings kept in `~/.brainbend-studio/assistant.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub api_key: String,
    pub model: String,
    /// The API's address; only a test sets it.
    pub base_url: Option<String>,
}

impl Default for Settings {
    fn default() -> Self { Self { api_key: String::new(), model: ai::MODELS[0].id.to_string(), base_url: None } }
}

impl Settings {
    fn path() -> Option<std::path::PathBuf> {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(std::path::PathBuf::from(home).join(".brainbend-studio").join("assistant.json"))
    }

    pub fn parse(text: &str) -> Self {
        let v: serde_json::Value = serde_json::from_str(text).unwrap_or(serde_json::Value::Null);
        let model = v["model"].as_str().filter(|m| ai::MODELS.iter().any(|k| k.id == *m)).unwrap_or(ai::MODELS[0].id);
        Settings { api_key: v["api_key"].as_str().unwrap_or("").trim().to_string(), model: model.to_string(), base_url: None }
    }

    pub fn to_json(&self) -> String { serde_json::json!({ "api_key": self.api_key, "model": self.model }).to_string() }

    pub fn load() -> Self { Self::path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Self::parse(&t)).unwrap_or_default() }

    pub fn save(&self) { if let Some(p) = Self::path() { mcp::write_private(&p, &self.to_json()); } }

    /// The key to use: the one the author saved, else `ANTHROPIC_API_KEY` from the environment. `None` when there is neither.
    pub fn key(&self) -> Option<String> {
        let saved = self.api_key.trim();
        if !saved.is_empty() { return Some(saved.to_string()); }
        std::env::var("ANTHROPIC_API_KEY").ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToolState { Running, Done, Failed(String) }

/// One line of the chat.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    You(String),
    Claude(String),
    Tool { name: String, what: String, state: ToolState },
    Notice(String),
    Error(String),
}

/// Tokens used in this chat, for an estimate of what it cost.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Usage { pub input: u64, pub output: u64, pub cache_read: u64, pub cache_write: u64 }

impl Usage {
    pub fn tokens(&self) -> u64 { self.input + self.output + self.cache_read + self.cache_write }

    /// US dollars at the model's list price: cached input costs a tenth, and writing the cache a quarter more (a rough
    /// estimate; the bill on the author's Anthropic account is what counts).
    pub fn dollars(&self, m: &ai::Model) -> f64 {
        (self.input as f64 * m.input + self.output as f64 * m.output + self.cache_read as f64 * m.input * 0.1 + self.cache_write as f64 * m.input * 1.25) / 1_000_000.0
    }
}

/// A few words about what a tool call asked for, for the chat.
fn what(name: &str, input: &serde_json::Value) -> String {
    let slide = input.get("slide").and_then(|s| s.as_u64()).map(|n| format!(" on slide {n}")).unwrap_or_default();
    match name {
        "get_project" | "list_slides" => "Looking at the lesson".into(),
        "get_slide" => format!("Looking at slide {}", input.get("slide").and_then(|s| s.as_u64()).unwrap_or(0)),
        "read" => format!("Reading {}", input.get("path").and_then(|p| p.as_str()).filter(|p| !p.is_empty()).unwrap_or("the lesson")),
        "describe_model" => format!("Checking what {} can have", input.get("type").and_then(|t| t.as_str()).unwrap_or("the lesson")),
        "add_slide" => "Adding a slide".into(),
        "delete_slide" => format!("Deleting a slide{slide}"),
        "rename_slide" | "set_slide_notes" | "update_slide" => format!("Changing a slide{slide}"),
        "add_text" | "add_shape" | "create_object" => format!("Adding an object{slide}"),
        "set_object" | "update_object" => format!("Changing an object{slide}"),
        "delete_object" => format!("Deleting an object{slide}"),
        "patch" => "Making changes".into(),
        "undo" => "Undoing".into(),
        "redo" => "Redoing".into(),
        other => other.to_string(),
    }
}

/// What came back from a tool, cut to a line for the chat.
fn brief(text: &str) -> String {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    if line.chars().count() > 100 { format!("{}…", line.chars().take(100).collect::<String>()) } else { line.to_string() }
}

/// Turns what the conversation reports into lines of the chat.
fn apply(chat: &mut Vec<Line>, usage: &mut Usage, event: ai::Event) {
    match event {
        ai::Event::Text(t) => chat.push(Line::Claude(t)),
        ai::Event::ToolStart { name, input } => { let w = what(&name, &input); chat.push(Line::Tool { name, what: w, state: ToolState::Running }); }
        ai::Event::ToolEnd { name, ok, text } => {
            if let Some(Line::Tool { state, .. }) = chat.iter_mut().rev().find(|l| matches!(l, Line::Tool { name: n, state: ToolState::Running, .. } if *n == name)) {
                *state = if ok { ToolState::Done } else { ToolState::Failed(brief(&text)) };
            }
        }
        ai::Event::Notice(n) => chat.push(Line::Notice(n)),
        ai::Event::Usage { input, output, cache_read, cache_write } => {
            usage.input += input; usage.output += output; usage.cache_read += cache_read; usage.cache_write += cache_write;
        }
    }
}

/// Claude's hands: the app's tools, run on the app's own thread.
struct UiTools {
    calls: Sender<ToolCall>,
    ctx: egui::Context,
    cancel: Arc<AtomicBool>,
}

impl ai::Tools for UiTools {
    fn definitions(&self) -> Vec<serde_json::Value> {
        mcp_tools::tools().into_iter().map(|t| serde_json::json!({ "name": t.name, "description": t.description, "input_schema": t.schema })).collect()
    }

    fn call(&mut self, name: &str, input: &serde_json::Value) -> Result<String, String> {
        let (reply, answer) = channel();
        self.calls.send(ToolCall { name: name.to_string(), arguments: input.clone(), reply }).map_err(|_| "BrainBend Studio is closing.".to_string())?;
        self.ctx.request_repaint();
        loop {
            match answer.recv_timeout(Duration::from_millis(200)) {
                Ok(result) => return result,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) if !self.cancel.load(Ordering::Relaxed) => {}
                Err(_) => return Err("Stopped.".into()),
            }
        }
    }
}

enum Msg {
    Event(ai::Event),
    Finished(Result<(), String>),
}

struct Run {
    rx: Receiver<Msg>,
    cancel: Arc<AtomicBool>,
    /// A tool was used during this message, so that an error can say that its changes are still there.
    used_tools: bool,
}

pub struct Assistant {
    pub settings: Settings,
    pub chat: Vec<Line>,
    pub input: String,
    pub usage: Usage,
    /// The text box for a new key.
    pub key_input: String,
    convo: Arc<Mutex<ai::Conversation>>,
    run: Option<Run>,
    calls_tx: Sender<ToolCall>,
    calls_rx: Receiver<ToolCall>,
    /// Scroll the chat to its end on the next frame.
    stick: bool,
}

impl Default for Assistant {
    fn default() -> Self {
        let (calls_tx, calls_rx) = channel();
        Self { settings: Settings::default(), chat: vec![], input: String::new(), usage: Usage::default(), key_input: String::new(), convo: Arc::new(Mutex::new(ai::Conversation::default())), run: None, calls_tx, calls_rx, stick: true }
    }
}

impl Assistant {
    pub fn load() -> Self { Self { settings: Settings::load(), ..Self::default() } }

    pub fn busy(&self) -> bool { self.run.is_some() }

    /// Starts over: an empty chat and a fresh conversation. Not while Claude is working.
    pub fn new_chat(&mut self) {
        if self.busy() { return; }
        self.chat.clear();
        self.usage = Usage::default();
        *self.convo.lock().unwrap_or_else(|e| e.into_inner()) = ai::Conversation::default();
    }

    pub fn stop(&mut self) { if let Some(r) = &self.run { r.cancel.store(true, Ordering::Relaxed); } }

    /// Sends the author's message to Claude, which works on its own thread while the app keeps drawing.
    pub fn send(&mut self, text: &str, ctx: &egui::Context) {
        let text = text.trim();
        if text.is_empty() || self.busy() { return; }
        let Some(key) = self.settings.key() else { self.chat.push(Line::Error("There is no API key. Add one first.".into())); return };
        self.chat.push(Line::You(text.to_string()));
        self.stick = true;
        let mut cfg = ai::Config::new(&key, &self.settings.model, &format!("{SYSTEM}{}", mcp_tools::INSTRUCTIONS));
        if let Some(u) = &self.settings.base_url { cfg.base_url = u.clone(); cfg.retry_ms = 0; }
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let (convo, ctx, flag, message) = (self.convo.clone(), ctx.clone(), cancel.clone(), text.to_string());
        let mut tools = UiTools { calls: self.calls_tx.clone(), ctx: ctx.clone(), cancel: cancel.clone() };
        std::thread::spawn(move || {
            let mut convo = convo.lock().unwrap_or_else(|e| e.into_inner());
            let events = tx.clone();
            let wake = ctx.clone();
            let result = ai::run_turn(&cfg, &mut convo, &message, &mut tools, &flag, &mut |e| { let _ = events.send(Msg::Event(e)); wake.request_repaint(); });
            let _ = tx.send(Msg::Finished(result));
            ctx.request_repaint();
        });
        self.run = Some(Run { rx, cancel, used_tools: false });
        self.input.clear();
    }

    /// Takes in what the conversation has reported since the last frame.
    fn poll(&mut self) {
        let mut finished: Option<Result<(), String>> = None;
        if let Some(run) = self.run.as_mut() {
            while let Ok(msg) = run.rx.try_recv() {
                match msg {
                    Msg::Event(e) => {
                        if matches!(e, ai::Event::ToolStart { .. }) { run.used_tools = true; }
                        apply(&mut self.chat, &mut self.usage, e);
                        self.stick = true;
                    }
                    Msg::Finished(r) => finished = Some(r),
                }
            }
        }
        if let Some(result) = finished {
            let used = self.run.take().is_some_and(|r| r.used_tools);
            match result {
                Ok(()) => {}
                Err(e) if e == "Stopped." => self.chat.push(Line::Notice(if used { "Stopped. What it had already changed is still there; Undo takes it back.".into() } else { "Stopped.".into() })),
                Err(e) => self.chat.push(Line::Error(if used { format!("{e} What it had already changed is still there; Undo takes it back.") } else { e })),
            }
            self.stick = true;
        }
    }
}

impl App {
    /// Once a frame: run the tools Claude asked for, and take in what it said.
    pub(super) fn assistant_frame(&mut self) {
        while let Ok(call) = self.assistant.calls_rx.try_recv() {
            let result = self.mcp_dispatch(&call.name, &call.arguments);
            self.mcp.note(&format!("assistant: {}", call.name), &result);
            let _ = call.reply.send(result);
        }
        self.assistant.poll();
    }

    /// The chat, at the top of the AI tab.
    pub(super) fn chat_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Ask Claude").strong());
            let current = ai::model(&self.assistant.settings.model);
            egui::ComboBox::from_id_salt("assistant-model").selected_text(current.name).show_ui(ui, |ui| {
                for m in ai::MODELS {
                    if ui.selectable_label(m.id == current.id, format!("{}  ({})", m.name, m.note)).clicked() && m.id != current.id { self.assistant.settings.model = m.id.to_string(); self.assistant.settings.save(); }
                }
            });
        });
        let Some(_) = self.assistant.settings.key() else {
            ui.label("Describe what you want and Claude builds it in your lesson: slides, text, shapes, styles, animations, timing and triggers. You watch it happen, and Undo takes any of it back.");
            ui.add_space(4.0);
            ui.label(egui::RichText::new("This needs your own Claude API key.").strong());
            ui.weak("What you type, and the parts of your lesson that Claude reads to do the work, are sent to Anthropic and billed to your key. The key is kept on this computer, readable only by you.");
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.assistant.key_input).password(true).hint_text("sk-ant-…").desired_width(180.0));
                let ready = !self.assistant.key_input.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new("Save key")).clicked() {
                    self.assistant.settings.api_key = self.assistant.key_input.trim().to_string();
                    self.assistant.key_input.clear();
                    self.assistant.settings.save();
                }
            });
            if ui.link("Get an API key at console.anthropic.com").clicked() { update::open_in_browser("https://console.anthropic.com/settings/keys"); }
            return;
        };
        let busy = self.assistant.busy();
        let model = ai::model(&self.assistant.settings.model);
        if !self.mcp.settings.allow_edit { ui.colored_label(ui.visuals().warn_fg_color, "Changes are switched off below, so Claude can look but not change anything."); }
        egui::ScrollArea::vertical().id_salt("assistant-chat").max_height(300.0).auto_shrink([false, true]).stick_to_bottom(self.assistant.stick).show(ui, |ui| {
            if self.assistant.chat.is_empty() {
                ui.weak("Ask for something, or start with one of these:");
                for s in ["Make a title slide for a workplace safety course", "Make every title on every slide bigger and bold", "Fade in the text on this slide, one box after another", "Add a slide that sums up the lesson in three points"] {
                    if ui.button(s).clicked() { self.assistant.input = s.to_string(); }
                }
            }
            for line in &self.assistant.chat {
                match line {
                    Line::You(t) => { ui.label(egui::RichText::new(format!("You: {t}")).strong()); }
                    Line::Claude(t) => { ui.label(t); }
                    Line::Tool { what, state, .. } => {
                        let (mark, color) = match state { ToolState::Running => ("…", ui.visuals().weak_text_color()), ToolState::Done => ("✓", ui.visuals().weak_text_color()), ToolState::Failed(_) => ("✗", egui::Color32::from_rgb(220, 70, 70)) };
                        let text = match state { ToolState::Failed(why) => format!("{mark} {what}: {why}"), _ => format!("{mark} {what}") };
                        ui.label(egui::RichText::new(text).small().color(color));
                    }
                    Line::Notice(t) => { ui.weak(t); }
                    Line::Error(t) => { ui.colored_label(egui::Color32::from_rgb(220, 70, 70), t); }
                }
                ui.add_space(2.0);
            }
            if busy { ui.horizontal(|ui| { ui.spinner(); ui.weak("Claude is working…"); }); }
        });
        self.assistant.stick = busy;
        let id = ui.make_persistent_id("assistant-input");
        // Enter sends and Shift+Enter starts a new line; taking the key before the box sees it keeps the line break out.
        let send_key = ui.memory(|m| m.has_focus(id)) && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        ui.add_enabled(!busy, egui::TextEdit::multiline(&mut self.assistant.input).id(id).desired_rows(3).desired_width(f32::INFINITY).hint_text("Ask for a change…  (Enter sends, Shift+Enter for a new line)"));
        ui.horizontal_wrapped(|ui| {
            if busy {
                if ui.button("Stop").clicked() { self.assistant.stop(); }
            } else {
                let can = !self.assistant.input.trim().is_empty();
                if (ui.add_enabled(can, egui::Button::new("Send")).clicked() || (send_key && can)) && can { let t = self.assistant.input.clone(); self.assistant.send(&t, &ctx); }
                if ui.add_enabled(!self.assistant.chat.is_empty(), egui::Button::new("New chat")).on_hover_text("Clear the chat and start over").clicked() { self.assistant.new_chat(); }
            }
            if ui.small_button("Remove key").on_hover_text("Forget the API key kept on this computer").clicked() && !busy { self.assistant.settings.api_key.clear(); self.assistant.settings.save(); }
        });
        let u = self.assistant.usage;
        if u.tokens() > 0 { ui.weak(format!("{} tokens this chat, about ${:.2} at {}'s list price (an estimate).", u.tokens(), u.dollars(&model), model.name)); }
        else { ui.weak(format!("{}. Billed to your Claude account.", model.name)); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_keep_the_key_and_model_and_fall_back_safely() {
        let s = Settings { api_key: "sk-ant-abc".into(), model: "claude-sonnet-5-5".into(), base_url: None };
        assert_eq!(Settings::parse(&s.to_json()), s);
        assert_eq!(Settings::parse("{broken").model, ai::MODELS[0].id, "a damaged file gives the usual model");
        assert_eq!(Settings::parse(r#"{"model":"gpt-9"}"#).model, ai::MODELS[0].id, "an unknown model is not kept");
        assert_eq!(Settings::parse(r#"{"api_key":"  k  "}"#).api_key, "k");
        assert!(!s.to_json().contains("base_url"), "the test address is never saved");
    }

    #[test]
    fn a_saved_key_wins_over_the_environment() {
        let s = Settings { api_key: "saved".into(), ..Default::default() };
        assert_eq!(s.key().as_deref(), Some("saved"));
    }

    #[test]
    fn events_become_chat_lines_and_a_tool_is_marked_done_or_failed() {
        let mut chat = vec![];
        let mut usage = Usage::default();
        apply(&mut chat, &mut usage, ai::Event::Text("Let me look.".into()));
        apply(&mut chat, &mut usage, ai::Event::ToolStart { name: "get_slide".into(), input: serde_json::json!({ "slide": 2 }) });
        apply(&mut chat, &mut usage, ai::Event::ToolStart { name: "add_text".into(), input: serde_json::json!({ "slide": 2 }) });
        apply(&mut chat, &mut usage, ai::Event::ToolEnd { name: "get_slide".into(), ok: true, text: "{}".into() });
        apply(&mut chat, &mut usage, ai::Event::ToolEnd { name: "add_text".into(), ok: false, text: "\n  The author is typing in a text box.\nmore".into() });
        apply(&mut chat, &mut usage, ai::Event::Notice("Stopped after 40 steps.".into()));
        apply(&mut chat, &mut usage, ai::Event::Usage { input: 1000, output: 200, cache_read: 5000, cache_write: 300 });
        apply(&mut chat, &mut usage, ai::Event::Usage { input: 100, output: 50, cache_read: 0, cache_write: 0 });
        assert_eq!(chat[0], Line::Claude("Let me look.".into()));
        assert_eq!(chat[1], Line::Tool { name: "get_slide".into(), what: "Looking at slide 2".into(), state: ToolState::Done });
        assert_eq!(chat[2], Line::Tool { name: "add_text".into(), what: "Adding an object on slide 2".into(), state: ToolState::Failed("The author is typing in a text box.".into()) });
        assert_eq!(chat[3], Line::Notice("Stopped after 40 steps.".into()));
        assert_eq!((usage.input, usage.output, usage.cache_read, usage.cache_write, usage.tokens()), (1100, 250, 5000, 300, 6650));
    }

    #[test]
    fn the_cost_estimate_uses_the_models_list_price() {
        let u = Usage { input: 1_000_000, output: 100_000, cache_read: 1_000_000, cache_write: 0 };
        let opus = ai::model("claude-opus-5-5");
        assert!((u.dollars(&opus) - (4.0 + 2.0 + 0.4)).abs() < 1e-9, "{}", u.dollars(&opus));
        assert!(u.dollars(&ai::model("claude-haiku-5-5")) < u.dollars(&opus) / 10.0);
        assert_eq!(Usage::default().dollars(&opus), 0.0);
    }

    #[test]
    fn a_tool_call_is_described_in_a_few_plain_words() {
        use serde_json::json as j;
        assert_eq!(what("list_slides", &j!({})), "Looking at the lesson");
        assert_eq!(what("create_object", &j!({ "slide": 3 })), "Adding an object on slide 3");
        assert_eq!(what("update_object", &j!({})), "Changing an object");
        assert_eq!(what("read", &j!({ "path": "/slides/0" })), "Reading /slides/0");
        assert_eq!(what("describe_model", &j!({ "type": "Entrance" })), "Checking what Entrance can have");
        assert_eq!(what("something_new", &j!({})), "something_new", "an unknown tool is shown by name");
        assert_eq!(brief("a\nb"), "a");
        assert!(brief(&"x".repeat(300)).chars().count() <= 101);
    }
}
