//! The chat in the AI tab: the author asks, an AI model builds and changes the lesson with the same tools an outside AI tool gets.
//!
//! The model (Claude, GPT or Gemini) is reached with the author's own API key for that provider, kept on this computer and
//! readable only by them; what the model reads of the lesson and what the author types go to that provider and are billed to
//! the key, which the tab says. (A Claude, ChatGPT or Gemini subscription cannot be used here: those companies do not let
//! other apps sign in with one. The tab points to their own tools, which connect through MCP, instead.) The conversation runs on
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

/// The settings kept in `~/.brainbend-studio/assistant.json`: which provider, and for each provider a key and a model.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub provider: ai::Provider,
    /// One key per provider, in the order of [`ai::PROVIDERS`]; empty for none.
    pub keys: [String; 3],
    /// One model id per provider; empty for the provider's usual one. Any id may be typed in.
    pub models: [String; 3],
    /// The API's address; only a test sets it.
    pub base_url: Option<String>,
}

impl Default for Settings {
    fn default() -> Self { Self { provider: ai::Provider::Anthropic, keys: Default::default(), models: Default::default(), base_url: None } }
}

impl Settings {
    fn path() -> Option<std::path::PathBuf> {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(std::path::PathBuf::from(home).join(".brainbend-studio").join("assistant.json"))
    }

    /// Reads the file. The first version kept only a Claude key and model, as `api_key` and `model`; those are still read.
    pub fn parse(text: &str) -> Self {
        let v: serde_json::Value = serde_json::from_str(text).unwrap_or(serde_json::Value::Null);
        let mut s = Settings::default();
        s.provider = v["provider"].as_str().and_then(ai::Provider::from_id).unwrap_or(ai::Provider::Anthropic);
        for p in ai::PROVIDERS {
            let i = p.index();
            s.keys[i] = v["keys"][p.id()].as_str().unwrap_or("").trim().to_string();
            s.models[i] = v["models"][p.id()].as_str().unwrap_or("").trim().to_string();
        }
        if s.keys[0].is_empty() { s.keys[0] = v["api_key"].as_str().unwrap_or("").trim().to_string(); }
        if s.models[0].is_empty() { s.models[0] = v["model"].as_str().unwrap_or("").trim().to_string(); }
        s
    }

    pub fn to_json(&self) -> String {
        let keys: serde_json::Map<String, serde_json::Value> = ai::PROVIDERS.iter().map(|p| (p.id().to_string(), self.keys[p.index()].clone().into())).collect();
        let models: serde_json::Map<String, serde_json::Value> = ai::PROVIDERS.iter().map(|p| (p.id().to_string(), self.models[p.index()].clone().into())).collect();
        serde_json::json!({ "provider": self.provider.id(), "keys": keys, "models": models }).to_string()
    }

    pub fn load() -> Self { Self::path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Self::parse(&t)).unwrap_or_default() }

    pub fn save(&self) { if let Some(p) = Self::path() { mcp::write_private(&p, &self.to_json()); } }

    /// Settings for Claude with this key.
    #[cfg(test)]
    pub fn claude(key: &str) -> Self { let mut s = Settings::default(); s.keys[0] = key.to_string(); s }

    /// The model in use: the one chosen for this provider, else its usual one.
    pub fn model(&self) -> String {
        let m = self.models[self.provider.index()].trim();
        if m.is_empty() { ai::default_model(self.provider).to_string() } else { m.to_string() }
    }

    pub fn set_model(&mut self, id: &str) { self.models[self.provider.index()] = id.trim().to_string(); }

    /// The key to use: the one the author saved for this provider, else the provider's variable in the environment. `None`
    /// when there is neither.
    pub fn key(&self) -> Option<String> {
        let saved = self.keys[self.provider.index()].trim();
        if !saved.is_empty() { return Some(saved.to_string()); }
        self.provider.env_keys().iter().find_map(|name| std::env::var(name).ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty()))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToolState { Running, Done, Failed(String) }

/// One line of the chat.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    You(String),
    Reply(String),
    Tool { name: String, what: String, state: ToolState },
    Notice(String),
    Error(String),
}

/// Tokens used in this chat, for an estimate of what it cost.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Usage { pub input: u64, pub output: u64, pub cache_read: u64, pub cache_write: u64 }

impl Usage {
    pub fn tokens(&self) -> u64 { self.input + self.output + self.cache_read + self.cache_write }

    /// US dollars at the model's list price (input, output per million tokens), when the price is known: cached input costs
    /// a tenth, and writing Claude's cache a quarter more (a rough estimate; the bill on the author's account is what counts).
    pub fn dollars(&self, price: Option<(f64, f64)>) -> Option<f64> {
        let (input, output) = price?;
        Some((self.input as f64 * input + self.output as f64 * output + self.cache_read as f64 * input * 0.1 + self.cache_write as f64 * input * 1.25) / 1_000_000.0)
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
        ai::Event::Text(t) => chat.push(Line::Reply(t)),
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

/// The model's hands: the app's tools, run on the app's own thread.
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
    /// The text box for a model id that is not in the list.
    pub model_input: String,
    /// The Set up AI window is open.
    pub setup_open: bool,
    /// What the window is showing: 0 an API key, 1 a subscription's own tool.
    pub setup_mode: usize,
    /// Which outside AI tool the window shows the command for.
    pub connect_tool: usize,
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
        Self { settings: Settings::default(), chat: vec![], input: String::new(), usage: Usage::default(), key_input: String::new(), model_input: String::new(), setup_open: false, setup_mode: 0, connect_tool: 0, convo: Arc::new(Mutex::new(ai::Conversation::default())), run: None, calls_tx, calls_rx, stick: true }
    }
}

impl Assistant {
    pub fn load() -> Self { Self { settings: Settings::load(), ..Self::default() } }

    pub fn busy(&self) -> bool { self.run.is_some() }

    /// Starts over: an empty chat and a fresh conversation. Not while the model is working.
    pub fn new_chat(&mut self) {
        if self.busy() { return; }
        self.chat.clear();
        self.usage = Usage::default();
        *self.convo.lock().unwrap_or_else(|e| e.into_inner()) = ai::Conversation::default();
    }

    /// Asks a different provider. A conversation is in one provider's form, so this starts a new one (not while working).
    pub fn switch_provider(&mut self, to: ai::Provider) {
        if self.busy() || to == self.settings.provider { return; }
        let had_chat = !self.chat.is_empty();
        self.new_chat();
        self.settings.provider = to;
        self.settings.save();
        if had_chat && self.settings.key().is_some() { self.chat.push(Line::Notice(format!("Now asking {}. This is a new chat; the lesson is as you left it.", to.name()))); }
    }

    pub fn stop(&mut self) { if let Some(r) = &self.run { r.cancel.store(true, Ordering::Relaxed); } }

    /// Sends the author's message to the model, which works on its own thread while the app keeps drawing.
    pub fn send(&mut self, text: &str, ctx: &egui::Context) {
        let text = text.trim();
        if text.is_empty() || self.busy() { return; }
        let Some(key) = self.settings.key() else { self.chat.push(Line::Error("There is no API key. Add one first.".into())); return };
        self.chat.push(Line::You(text.to_string()));
        self.stick = true;
        let mut cfg = ai::Config::new(self.settings.provider, &key, &self.settings.model(), &format!("{SYSTEM}{}", mcp_tools::INSTRUCTIONS));
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
    /// Once a frame: run the tools the model asked for, and take in what it said.
    pub(super) fn assistant_frame(&mut self) {
        while let Ok(call) = self.assistant.calls_rx.try_recv() {
            let result = self.mcp_dispatch(&call.name, &call.arguments);
            self.mcp.note(&format!("assistant: {}", call.name), &result);
            let _ = call.reply.send(result);
        }
        self.assistant.poll();
    }

    /// The provider list.
    pub(super) fn provider_picker(&mut self, ui: &mut egui::Ui) {
        let provider = self.assistant.settings.provider;
        let mut chosen = provider;
        ui.add_enabled_ui(!self.assistant.busy(), |ui| {
            egui::ComboBox::from_id_salt("assistant-provider").selected_text(provider.label()).show_ui(ui, |ui| {
                for p in ai::PROVIDERS { ui.selectable_value(&mut chosen, p, p.label()); }
            });
        });
        if chosen != provider { self.assistant.switch_provider(chosen); }
    }

    /// The list of the provider's models.
    pub(super) fn model_picker(&mut self, ui: &mut egui::Ui) {
        let provider = self.assistant.settings.provider;
        let current = self.assistant.settings.model();
        let known = ai::find_model(provider, &current);
        ui.add_enabled_ui(!self.assistant.busy(), |ui| {
            egui::ComboBox::from_id_salt(("assistant-model", provider.id())).selected_text(known.map(|m| m.name).unwrap_or(current.as_str())).show_ui(ui, |ui| {
                for m in provider.models() {
                    if ui.selectable_label(m.id == current, format!("{}  ({})", m.name, m.note)).clicked() && m.id != current { self.assistant.settings.set_model(m.id); self.assistant.settings.save(); }
                }
            });
        });
    }

    /// A box for a model id that is not in the list.
    pub(super) fn other_model_ui(&mut self, ui: &mut egui::Ui) {
        let busy = self.assistant.busy();
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.assistant.model_input).hint_text("Another model id").desired_width(160.0));
            if ui.add_enabled(!busy && !self.assistant.model_input.trim().is_empty(), egui::Button::new("Use")).on_hover_text("Used as typed; a wrong id is refused with an error").clicked() {
                let typed = self.assistant.model_input.trim().to_string();
                self.assistant.settings.set_model(&typed);
                self.assistant.model_input.clear();
                self.assistant.settings.save();
            }
        });
    }

    /// The chat, in the AI tab once a key is set.
    pub(super) fn chat_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let provider = self.assistant.settings.provider;
        let name = provider.name();
        let busy = self.assistant.busy();
        let model_id = self.assistant.settings.model();
        let known = ai::find_model(provider, &model_id);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            self.provider_picker(ui);
            self.model_picker(ui);
        });
        if !self.mcp.settings.allow_edit { ui.colored_label(ui.visuals().warn_fg_color, format!("Changes are off, so {name} can look but not change anything.")); }
        let room = (ui.available_height() - 150.0).max(150.0);
        egui::ScrollArea::vertical().id_salt("assistant-chat").max_height(room).auto_shrink([false, true]).stick_to_bottom(self.assistant.stick).show(ui, |ui| {
            if self.assistant.chat.is_empty() {
                ui.weak("Ask for something, or start with one of these:");
                for s in ["Make a title slide for a workplace safety course", "Make every title on every slide bigger and bold", "Fade in the text on this slide, one box after another", "Add a slide that sums up the lesson in three points"] {
                    if ui.button(s).clicked() { self.assistant.input = s.to_string(); }
                }
            }
            for line in &self.assistant.chat {
                match line {
                    Line::You(t) => { ui.label(egui::RichText::new(format!("You: {t}")).strong()); }
                    Line::Reply(t) => { ui.label(t); }
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
            if busy { ui.horizontal(|ui| { ui.spinner(); ui.weak(format!("{name} is working…")); }); }
        });
        self.assistant.stick = busy;
        let id = ui.make_persistent_id("assistant-input");
        // Enter sends and Shift+Enter starts a new line; taking the key before the box sees it keeps the line break out.
        let send_key = ui.memory(|m| m.has_focus(id)) && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        ui.add_enabled(!busy, egui::TextEdit::multiline(&mut self.assistant.input).id(id).desired_rows(3).desired_width(f32::INFINITY).hint_text("Ask for a change…"));
        ui.horizontal_wrapped(|ui| {
            if busy {
                if ui.button("Stop").clicked() { self.assistant.stop(); }
            } else {
                let can = !self.assistant.input.trim().is_empty();
                if (ui.add_enabled(can, egui::Button::new("Send")).clicked() || (send_key && can)) && can { let t = self.assistant.input.clone(); self.assistant.send(&t, &ctx); }
                if ui.add_enabled(!self.assistant.chat.is_empty(), egui::Button::new("New chat")).on_hover_text("Clear the chat and start over").clicked() { self.assistant.new_chat(); }
            }
            if ui.small_button("Settings").on_hover_text("Keys, models and connecting other AI tools").clicked() { self.assistant.setup_open = true; }
        });
        let u = self.assistant.usage;
        let shown = known.map(|m| m.name.to_string()).unwrap_or_else(|| model_id.clone());
        if u.tokens() > 0 {
            match u.dollars(known.and_then(|m| m.price)) {
                Some(d) => { ui.weak(format!("{} tokens, about ${d:.2} at {shown}'s list price.", u.tokens())); }
                None => { ui.weak(format!("{} tokens. No price is known for {shown}.", u.tokens())); }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_keep_a_key_and_model_for_each_provider_and_fall_back_safely() {
        let mut s = Settings::claude("sk-ant-abc");
        s.keys[ai::Provider::OpenAi.index()] = "sk-openai".into();
        s.keys[ai::Provider::Gemini.index()] = "AIza-g".into();
        s.models[0] = "claude-sonnet-5-5".into();
        s.models[ai::Provider::Gemini.index()] = "gemini-9-custom".into();
        s.provider = ai::Provider::Gemini;
        assert_eq!(Settings::parse(&s.to_json()), s);
        assert_eq!((s.model().as_str(), s.key().as_deref()), ("gemini-9-custom", Some("AIza-g")), "a typed model id is kept and used");
        s.provider = ai::Provider::OpenAi;
        assert_eq!(s.model(), ai::default_model(ai::Provider::OpenAi), "no choice gives the provider's usual model");
        assert_eq!(Settings::parse("{broken"), Settings::default());
        assert_eq!(Settings::parse(r#"{"provider":"nonsense"}"#).provider, ai::Provider::Anthropic);
        assert!(!s.to_json().contains("base_url"), "the test address is never saved");
    }

    #[test]
    fn a_file_from_before_there_were_other_providers_still_reads() {
        let s = Settings::parse(r#"{"api_key":"  k  ","model":"claude-haiku-5-5"}"#);
        assert_eq!((s.provider, s.keys[0].as_str(), s.model().as_str()), (ai::Provider::Anthropic, "k", "claude-haiku-5-5"));
        assert!(s.keys[1].is_empty() && s.keys[2].is_empty());
    }

    #[test]
    fn a_saved_key_wins_over_the_environment_and_each_provider_has_its_own() {
        let mut s = Settings::claude("saved");
        assert_eq!(s.key().as_deref(), Some("saved"));
        s.provider = ai::Provider::OpenAi;
        std::env::remove_var("OPENAI_API_KEY");
        assert_eq!(s.key(), None, "a Claude key is not offered to OpenAI");
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
        assert_eq!(chat[0], Line::Reply("Let me look.".into()));
        assert_eq!(chat[1], Line::Tool { name: "get_slide".into(), what: "Looking at slide 2".into(), state: ToolState::Done });
        assert_eq!(chat[2], Line::Tool { name: "add_text".into(), what: "Adding an object on slide 2".into(), state: ToolState::Failed("The author is typing in a text box.".into()) });
        assert_eq!(chat[3], Line::Notice("Stopped after 40 steps.".into()));
        assert_eq!((usage.input, usage.output, usage.cache_read, usage.cache_write, usage.tokens()), (1100, 250, 5000, 300, 6650));
    }

    #[test]
    fn the_cost_estimate_uses_the_models_list_price_and_is_none_when_the_price_is_not_known() {
        let u = Usage { input: 1_000_000, output: 100_000, cache_read: 1_000_000, cache_write: 0 };
        let price = |id: &str| ai::find_model(ai::Provider::Anthropic, id).and_then(|m| m.price);
        assert!((u.dollars(price("claude-opus-5-5")).unwrap() - (4.0 + 2.0 + 0.4)).abs() < 1e-9);
        assert!(u.dollars(price("claude-haiku-5-5")).unwrap() < u.dollars(price("claude-opus-5-5")).unwrap() / 10.0);
        assert_eq!(Usage::default().dollars(price("claude-opus-5-5")), Some(0.0));
        assert_eq!(u.dollars(None), None, "a model the author typed has no price here");
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
