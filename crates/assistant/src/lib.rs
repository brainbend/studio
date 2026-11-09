//! The assistant built into BrainBend Studio: a chat in which an AI model uses the app's tools to build and change a lesson.
//!
//! This crate knows nothing about slides. It speaks three providers' HTTP APIs (Claude, OpenAI and Gemini; there is no
//! official Rust SDK for any of them, so it is plain requests) and runs the tool-use loop: send the conversation and the
//! tools, run whichever tools the model asks for, send their results back, and repeat until it has finished. The app
//! supplies the tools ([`Tools`]) and shows the events ([`Event`]).
//!
//! Each provider has a small module that knows only its own wire format: how to build a request, how to read a reply into
//! a [`Step`], and how to put the author's message and the tool results into the conversation. The loop is shared.
//!
//! Things the APIs need that are easy to get wrong, and are done here:
//! - The model's reply is kept in the history exactly as it came (Claude's thinking blocks, Gemini's thought signatures),
//!   and every result of a turn's tool calls goes back together.
//! - A refusal is handled before anything in the reply is read, and a turn that fails is taken back out of the history, so
//!   one error cannot leave the conversation in a shape the API rejects.
//! - Optional Claude request features (a beta fallback model, prompt caching, effort) are dropped, and the request tried
//!   again, if the API says it does not take them, so a feature this build guessed wrong never stops the chat working.
//! - A conversation belongs to one provider: its messages are in that provider's form, so switching starts a new chat.
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

mod anthropic;
mod gemini;
mod openai;

/// The most requests one message from the author may cause, so a loop that never ends costs a bounded amount.
const MAX_STEPS: usize = 40;
const MAX_RETRIES: u32 = 3;

/// Whose model answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Anthropic,
    OpenAi,
    Gemini,
}

pub const PROVIDERS: [Provider; 3] = [Provider::Anthropic, Provider::OpenAi, Provider::Gemini];

impl Provider {
    /// Position in [`PROVIDERS`], for keeping one setting per provider.
    pub fn index(self) -> usize { PROVIDERS.iter().position(|p| *p == self).unwrap_or(0) }
    /// The word kept in settings files.
    pub fn id(self) -> &'static str { match self { Provider::Anthropic => "anthropic", Provider::OpenAi => "openai", Provider::Gemini => "gemini" } }
    pub fn from_id(id: &str) -> Option<Provider> { PROVIDERS.iter().copied().find(|p| p.id() == id) }
    /// What the assistant is called in the chat.
    pub fn name(self) -> &'static str { match self { Provider::Anthropic => "Claude", Provider::OpenAi => "GPT", Provider::Gemini => "Gemini" } }
    /// The name in the provider list.
    pub fn label(self) -> &'static str { match self { Provider::Anthropic => "Claude (Anthropic)", Provider::OpenAi => "GPT (OpenAI)", Provider::Gemini => "Gemini (Google)" } }
    /// The company that receives what is sent and bills the key.
    pub fn company(self) -> &'static str { match self { Provider::Anthropic => "Anthropic", Provider::OpenAi => "OpenAI", Provider::Gemini => "Google" } }
    /// Environment variables that may hold a key, best first.
    pub fn env_keys(self) -> &'static [&'static str] {
        match self { Provider::Anthropic => &["ANTHROPIC_API_KEY"], Provider::OpenAi => &["OPENAI_API_KEY"], Provider::Gemini => &["GEMINI_API_KEY", "GOOGLE_API_KEY"] }
    }
    /// Where the author makes a key.
    pub fn key_url(self) -> &'static str {
        match self { Provider::Anthropic => "https://console.anthropic.com/settings/keys", Provider::OpenAi => "https://platform.openai.com/api-keys", Provider::Gemini => "https://aistudio.google.com/apikey" }
    }
    /// The place on the web to look at a key's account and billing.
    pub fn billing_site(self) -> &'static str { match self { Provider::Anthropic => "console.anthropic.com", Provider::OpenAi => "platform.openai.com", Provider::Gemini => "aistudio.google.com" } }
    /// What a key starts with, as a hint in the box.
    pub fn key_hint(self) -> &'static str { match self { Provider::Anthropic => "sk-ant-…", Provider::OpenAi => "sk-…", Provider::Gemini => "AIza…" } }
    pub fn models(self) -> &'static [Model] { match self { Provider::Anthropic => &anthropic::MODELS, Provider::OpenAi => &openai::MODELS, Provider::Gemini => &gemini::MODELS } }
    fn default_base(self) -> &'static str {
        match self { Provider::Anthropic => "https://api.anthropic.com", Provider::OpenAi => "https://api.openai.com", Provider::Gemini => "https://generativelanguage.googleapis.com" }
    }
}

/// A model the author can pick from the list. Any other model id can be typed in; it just has no known price.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub note: &'static str,
    /// US dollars per million tokens, input and output, when known.
    pub price: Option<(f64, f64)>,
    /// Claude only: whether the API can re-run a declined request on a fallback model.
    pub fallback: bool,
}

/// The listed model with this id.
pub fn find_model(provider: Provider, id: &str) -> Option<Model> { provider.models().iter().copied().find(|m| m.id == id) }

/// The model to use when none is chosen.
pub fn default_model(provider: Provider) -> &'static str { provider.models()[0].id }

/// How to reach a model.
#[derive(Clone, Debug)]
pub struct Config {
    pub provider: Provider,
    pub api_key: String,
    pub model: String,
    /// The API's address; only a test changes it.
    pub base_url: String,
    pub max_tokens: u32,
    /// Waiting before a retry is this many milliseconds times 2 to the number of tries (the API's own `retry-after` wins).
    pub retry_ms: u64,
    pub system: String,
}

impl Config {
    pub fn new(provider: Provider, api_key: &str, model: &str, system: &str) -> Self {
        Self { provider, api_key: api_key.trim().to_string(), model: model.to_string(), base_url: provider.default_base().into(), max_tokens: 16_000, retry_ms: 1000, system: system.to_string() }
    }
}

/// What the app shows as a turn goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Words from the model.
    Text(String),
    /// The model is using a tool.
    ToolStart { name: String, input: Value },
    /// The tool finished: whether it worked, and what it said.
    ToolEnd { name: String, ok: bool, text: String },
    /// Something the author should know that is not an error.
    Notice(String),
    /// Tokens used by one request.
    Usage { input: u64, output: u64, cache_read: u64, cache_write: u64 },
}

/// What the model can do for the app: the tools, and running one.
pub trait Tools {
    /// Each tool as the app describes it: `name`, `description` and `input_schema` (a JSON Schema).
    fn definitions(&self) -> Vec<Value>;
    /// Runs a tool: what it says, or why it could not.
    fn call(&mut self, name: &str, input: &Value) -> Result<String, String>;
}

/// The conversation so far, in the form its provider's API takes.
#[derive(Clone, Debug, Default)]
pub struct Conversation {
    pub messages: Vec<Value>,
}

/// Optional parts of a Claude request, each dropped if the API does not take it.
#[derive(Clone, Copy)]
struct Features { fallback: bool, cache: bool, effort: bool }

/// One request, ready to send.
struct Request {
    url: String,
    headers: Vec<(&'static str, String)>,
    body: Value,
}

/// A tool the model asked for. Its input is an error when the model wrote arguments that cannot be read.
struct Call {
    id: String,
    name: String,
    input: Result<Value, String>,
}

/// A tool's outcome, on its way back to the model.
struct Outcome {
    id: String,
    name: String,
    ok: bool,
    text: String,
}

/// Why a reply ended.
enum End {
    /// The model has finished.
    Done,
    /// The model wants its tools run.
    Tools,
    /// The reply ran out of room.
    Cut,
    /// The model or a safety filter declined; the text says why when it said.
    Refused(String),
    /// The conversation no longer fits.
    TooLong,
    /// The reply was unusable; the text is what to tell the author.
    Bad(String),
}

#[derive(Default)]
struct Tokens { input: u64, output: u64, cache_read: u64, cache_write: u64 }

/// One reply, read.
struct Step {
    /// What to keep in the conversation; `None` when the reply is not to be kept.
    message: Option<Value>,
    texts: Vec<String>,
    calls: Vec<Call>,
    tokens: Tokens,
    end: End,
}

fn n(v: &Value) -> u64 { v.as_u64().unwrap_or(0) }

/// The text of an error reply, or what the status means.
fn explain(provider: Provider, status: u16, body: &str) -> String {
    let api = serde_json::from_str::<Value>(body).ok().and_then(|v| v["error"]["message"].as_str().map(str::to_string));
    let who = provider.name();
    let base = match status {
        401 => "The API key was not accepted. Check it in the AI tab.".to_string(),
        402 => format!("Your {} account has a billing problem. Check it at {}.", provider.company(), provider.billing_site()),
        403 => "This API key is not allowed to do that.".to_string(),
        404 => "That model is not available to this API key. Pick another, or type a model id.".to_string(),
        413 => "The conversation is too big to send. Start a new chat.".to_string(),
        429 => format!("{who} is busy or this key has hit its rate limit or spending limit. Wait a moment and try again."),
        500..=599 => format!("{who} is having trouble right now. Try again in a moment."),
        _ => format!("{who} could not take the request."),
    };
    match api { Some(m) if !m.is_empty() => format!("{base} ({m})"), _ => base }
}

fn retryable(status: u16) -> bool { matches!(status, 429 | 500 | 502 | 503 | 504 | 529) }

/// One request, retried when the API is busy. Returns the reply, or why it failed (with the status for a 4xx).
fn post(cfg: &Config, req: &Request, cancel: &AtomicBool) -> Result<Value, (Option<u16>, String)> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(600))).http_status_as_error(false).build().into();
    let mut tries = 0;
    loop {
        if cancel.load(Ordering::Relaxed) { return Err((None, "Stopped.".into())); }
        let mut builder = agent.post(&req.url).header("content-type", "application/json");
        for (k, v) in &req.headers { builder = builder.header(*k, v.as_str()); }
        let sent = builder.send(req.body.to_string().as_bytes());
        let (status, text, wait) = match sent {
            Ok(mut reply) => {
                let status = reply.status().as_u16();
                let wait = reply.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<u64>().ok());
                (status, reply.body_mut().read_to_string().unwrap_or_default(), wait)
            }
            Err(e) => (0, e.to_string(), None),
        };
        if status == 200 { return serde_json::from_str(&text).map_err(|e| (None, format!("{}'s reply could not be read: {e}", cfg.provider.name()))); }
        let transport = status == 0;
        if (transport || retryable(status)) && tries < MAX_RETRIES {
            tries += 1;
            let ms = wait.map(|s| (s * 1000).min(20_000)).unwrap_or(cfg.retry_ms << (tries - 1));
            let mut left = ms;
            while left > 0 && !cancel.load(Ordering::Relaxed) { let step = left.min(100); std::thread::sleep(Duration::from_millis(step)); left -= step; }
            continue;
        }
        return Err(if transport { (None, format!("Could not reach {}. Check the internet connection.", cfg.provider.name())) } else { (Some(status), explain(cfg.provider, status, &text)) });
    }
}

/// Which optional Claude feature a 400 is about, if it names one.
fn rejected(message: &str, features: Features) -> Option<Features> {
    let m = message.to_ascii_lowercase();
    if features.fallback && (m.contains("fallback") || m.contains("anthropic-beta")) { return Some(Features { fallback: false, ..features }); }
    if features.cache && m.contains("cache_control") { return Some(Features { cache: false, ..features }); }
    if features.effort && (m.contains("effort") || m.contains("output_config")) { return Some(Features { effort: false, ..features }); }
    None
}

fn user_message(provider: Provider, text: &str) -> Value {
    match provider {
        Provider::Gemini => json!({ "role": "user", "parts": [{ "text": text }] }),
        _ => json!({ "role": "user", "content": text }),
    }
}

fn request(cfg: &Config, defs: &[Value], messages: &[Value], features: Features) -> Request {
    match cfg.provider {
        Provider::Anthropic => anthropic::request(cfg, defs, messages, features),
        Provider::OpenAi => openai::request(cfg, defs, messages),
        Provider::Gemini => gemini::request(cfg, defs, messages),
    }
}

fn read(provider: Provider, reply: &Value) -> Step {
    match provider { Provider::Anthropic => anthropic::read(reply), Provider::OpenAi => openai::read(reply), Provider::Gemini => gemini::read(reply) }
}

/// The messages that carry a turn's tool results back: every result of the turn, together where the API wants that.
fn results_messages(provider: Provider, results: &[Outcome]) -> Vec<Value> {
    match provider { Provider::Anthropic => anthropic::results(results), Provider::OpenAi => openai::results(results), Provider::Gemini => gemini::results(results) }
}

/// Runs one message from the author to the end: asks the model, runs the tools it asks for, and repeats. `emit` gets each
/// event as it happens. On any failure the turn is taken back out of the conversation, so the next message starts clean.
pub fn run_turn(cfg: &Config, convo: &mut Conversation, text: &str, tools: &mut dyn Tools, cancel: &AtomicBool, emit: &mut dyn FnMut(Event)) -> Result<(), String> {
    let start = convo.messages.len();
    let result = turn(cfg, convo, text, tools, cancel, emit);
    if result.is_err() { convo.messages.truncate(start); }
    result
}

fn turn(cfg: &Config, convo: &mut Conversation, text: &str, tools: &mut dyn Tools, cancel: &AtomicBool, emit: &mut dyn FnMut(Event)) -> Result<(), String> {
    if cfg.api_key.trim().is_empty() { return Err("There is no API key. Add one in the AI tab.".into()); }
    let who = cfg.provider.name();
    let defs = tools.definitions();
    let claude = cfg.provider == Provider::Anthropic;
    let mut features = Features { fallback: claude && find_model(cfg.provider, &cfg.model).is_some_and(|m| m.fallback), cache: claude, effort: claude };
    convo.messages.push(user_message(cfg.provider, text));
    for _ in 0..MAX_STEPS {
        let reply = loop {
            match post(cfg, &request(cfg, &defs, &convo.messages, features), cancel) {
                Ok(r) => break r,
                Err((Some(400), message)) => match rejected(&message, features) { Some(f) => features = f, None => return Err(message) },
                Err((_, message)) => return Err(message),
            }
        };
        let step = read(cfg.provider, &reply);
        let t = &step.tokens;
        emit(Event::Usage { input: t.input, output: t.output, cache_read: t.cache_read, cache_write: t.cache_write });
        // A refusal is read first: nothing in a refused reply is to be run or kept.
        match &step.end {
            End::Refused(why) => {
                let why = if why.is_empty() { String::new() } else { format!(" ({why})") };
                return Err(format!("{who} declined this request{why}. Try asking for it another way."));
            }
            End::Bad(message) => return Err(message.clone()),
            _ => {}
        }
        // Kept exactly as it came: Claude's thinking blocks and Gemini's thought signatures must go back unchanged.
        if let Some(m) = step.message { convo.messages.push(m); }
        for t in &step.texts { if !t.trim().is_empty() { emit(Event::Text(t.clone())); } }
        match step.end {
            End::Tools if !step.calls.is_empty() => {
                let mut results = vec![];
                for call in step.calls {
                    if cancel.load(Ordering::Relaxed) { return Err("Stopped.".into()); }
                    let (ok, out) = match &call.input {
                        Ok(input) => {
                            emit(Event::ToolStart { name: call.name.clone(), input: input.clone() });
                            match tools.call(&call.name, input) { Ok(t) => (true, t), Err(e) => (false, e) }
                        }
                        Err(e) => {
                            emit(Event::ToolStart { name: call.name.clone(), input: Value::Null });
                            (false, format!("The arguments could not be read as JSON ({e}). Send them again as one JSON object."))
                        }
                    };
                    emit(Event::ToolEnd { name: call.name.clone(), ok, text: out.clone() });
                    results.push(Outcome { id: call.id, name: call.name, ok, text: out });
                }
                convo.messages.extend(results_messages(cfg.provider, &results));
            }
            End::Cut => {
                if !step.calls.is_empty() { return Err(format!("{who}'s reply was cut off in the middle of a step. Try a smaller request.")); }
                emit(Event::Notice("The reply was cut short. Say \"continue\" to go on.".into()));
                return Ok(());
            }
            End::TooLong => return Err(format!("The conversation is too long for {who}. Start a new chat.")),
            _ => return Ok(()),
        }
    }
    emit(Event::Notice(format!("Stopped after {MAX_STEPS} steps. Say \"continue\" to go on.")));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// What the stand-in was sent: the headers and the body of each request.
    type Seen = Arc<Mutex<Vec<(Vec<(String, String)>, Value)>>>;
    /// A scripted answer: its status, extra headers and body.
    type Reply = (u16, Vec<(&'static str, &'static str)>, Value);

    /// A stand-in for the API: it answers each request with the next scripted reply, and keeps what it was sent.
    struct Fake {
        url: String,
        seen: Seen,
    }

    fn fake(replies: Vec<Reply>) -> Fake {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}", server.server_addr().to_ip().unwrap().port());
        let seen = Arc::new(Mutex::new(vec![]));
        let log = seen.clone();
        std::thread::spawn(move || {
            for (status, headers, reply) in replies {
                let Ok(mut req) = server.recv() else { return };
                let mut raw = String::new();
                std::io::Read::read_to_string(req.as_reader(), &mut raw).unwrap();
                let mut sent: Vec<(String, String)> = req.headers().iter().map(|h| (h.field.as_str().as_str().to_ascii_lowercase(), h.value.as_str().to_string())).collect();
                sent.push((":path".into(), req.url().to_string()));
                log.lock().unwrap().push((sent, serde_json::from_str(&raw).unwrap_or(Value::Null)));
                let mut r = tiny_http::Response::from_string(reply.to_string()).with_status_code(status);
                r.add_header(tiny_http::Header::from_bytes(&b"content-type"[..], &b"application/json"[..]).unwrap());
                for (k, v) in headers { r.add_header(tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()); }
                let _ = req.respond(r);
            }
        });
        Fake { url, seen }
    }

    fn cfg(f: &Fake) -> Config {
        let mut c = Config::new(Provider::Anthropic, "sk-test", "claude-opus-5-5", "You help build lessons.");
        c.base_url = f.url.clone();
        c.retry_ms = 0;
        c
    }

    struct Recorder { calls: Vec<(String, Value)>, fail: Vec<&'static str> }
    impl Tools for Recorder {
        fn definitions(&self) -> Vec<Value> { vec![json!({ "name": "get_project", "description": "Facts", "input_schema": { "type": "object", "properties": {} } })] }
        fn call(&mut self, name: &str, input: &Value) -> Result<String, String> {
            self.calls.push((name.to_string(), input.clone()));
            if self.fail.contains(&name) { Err("it did not work".into()) } else { Ok(format!("result of {name}")) }
        }
    }

    fn recorder() -> Recorder { Recorder { calls: vec![], fail: vec![] } }

    fn end(text: &str) -> Value { json!({ "id": "m", "type": "message", "role": "assistant", "stop_reason": "end_turn", "content": [{ "type": "text", "text": text }], "usage": { "input_tokens": 10, "output_tokens": 5 } }) }

    fn tool_use(blocks: Vec<Value>) -> Value { json!({ "id": "m", "role": "assistant", "stop_reason": "tool_use", "content": blocks, "usage": { "input_tokens": 100, "output_tokens": 20, "cache_read_input_tokens": 80 } }) }

    fn run(c: &Config, convo: &mut Conversation, text: &str, t: &mut Recorder) -> (Result<(), String>, Vec<Event>) {
        let mut events = vec![];
        let r = run_turn(c, convo, text, t, &AtomicBool::new(false), &mut |e| events.push(e));
        (r, events)
    }

    #[test]
    fn a_plain_answer_is_one_request_with_the_documented_shape() {
        let f = fake(vec![(200, vec![], end("Hello!"))]);
        let mut convo = Conversation::default();
        let (r, events) = run(&cfg(&f), &mut convo, "Hi", &mut recorder());
        assert!(r.is_ok());
        assert_eq!(events, vec![Event::Usage { input: 10, output: 5, cache_read: 0, cache_write: 0 }, Event::Text("Hello!".into())]);
        let (headers, body) = f.seen.lock().unwrap()[0].clone();
        let header = |k: &str| headers.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        assert_eq!((header("x-api-key").as_deref(), header("anthropic-version").as_deref(), header("anthropic-beta").as_deref()), (Some("sk-test"), Some("2026-10-21"), Some(anthropic::FALLBACK_BETA)));
        assert_eq!((body["model"].as_str(), body["max_tokens"].as_u64(), body["system"].as_str()), (Some("claude-opus-5-5"), Some(16_000), Some("You help build lessons.")));
        assert_eq!(body["tools"][0]["name"], "get_project");
        assert_eq!(body["messages"], json!([{ "role": "user", "content": "Hi" }]));
        assert_eq!((&body["fallbacks"], &body["cache_control"], &body["output_config"]), (&json!("default"), &json!({ "type": "ephemeral" }), &json!({ "effort": "medium" })));
        assert!(body.get("thinking").is_none() && body.get("tool_choice").is_none() && body.get("temperature").is_none(), "nothing the newest models reject");
        assert_eq!(convo.messages.len(), 2, "the question and the answer are kept");
    }

    #[test]
    fn tools_are_run_and_their_results_go_back_in_one_message_with_the_replys_thinking_intact() {
        let thinking = json!({ "type": "thinking", "thinking": "", "signature": "sig-abc" });
        let f = fake(vec![
            (200, vec![], tool_use(vec![
                thinking.clone(),
                json!({ "type": "text", "text": "Let me look." }),
                json!({ "type": "tool_use", "id": "toolu_1", "name": "get_project", "input": {} }),
                json!({ "type": "tool_use", "id": "toolu_2", "name": "get_project", "input": { "again": true } }),
            ])),
            (200, vec![], end("Two slides.")),
        ]);
        let mut convo = Conversation::default();
        let mut t = recorder();
        let (r, events) = run(&cfg(&f), &mut convo, "What is here?", &mut t);
        assert!(r.is_ok());
        assert_eq!(t.calls.len(), 2);
        let kinds: Vec<&str> = events.iter().map(|e| match e { Event::Text(_) => "text", Event::ToolStart { .. } => "start", Event::ToolEnd { .. } => "end", Event::Usage { .. } => "usage", Event::Notice(_) => "notice" }).collect();
        assert_eq!(kinds, vec!["usage", "text", "start", "end", "start", "end", "usage", "text"]);
        assert!(events.contains(&Event::Usage { input: 100, output: 20, cache_read: 80, cache_write: 0 }));
        let second = f.seen.lock().unwrap()[1].1.clone();
        let messages = second["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"][0], thinking, "the thinking block came back unchanged");
        assert_eq!(messages[2]["role"], "user");
        let results = messages[2]["content"].as_array().unwrap();
        assert_eq!(results.len(), 2, "both results in one message");
        assert_eq!((results[0]["type"].as_str(), results[0]["tool_use_id"].as_str(), results[0]["content"].as_str()), (Some("tool_result"), Some("toolu_1"), Some("result of get_project")));
        assert_eq!(results[1]["tool_use_id"], "toolu_2");
        assert!(results[0].get("is_error").is_none());
    }

    #[test]
    fn a_tool_that_fails_is_reported_to_claude_as_an_error_and_the_turn_goes_on() {
        let f = fake(vec![
            (200, vec![], tool_use(vec![json!({ "type": "tool_use", "id": "toolu_1", "name": "get_project", "input": {} })])),
            (200, vec![], end("That did not work, so I will stop.")),
        ]);
        let mut t = recorder();
        t.fail = vec!["get_project"];
        let mut convo = Conversation::default();
        let (r, events) = run(&cfg(&f), &mut convo, "Go", &mut t);
        assert!(r.is_ok());
        assert!(events.contains(&Event::ToolEnd { name: "get_project".into(), ok: false, text: "it did not work".into() }));
        let sent = f.seen.lock().unwrap()[1].1.clone();
        assert_eq!(sent["messages"][2]["content"][0]["is_error"], true);
        assert_eq!(sent["messages"][2]["content"][0]["content"], "it did not work");
    }

    #[test]
    fn a_refusal_is_not_run_or_kept_and_the_turn_is_taken_back() {
        let refusal = json!({ "role": "assistant", "stop_reason": "refusal", "stop_details": { "type": "refusal", "category": "cyber", "explanation": "not that" }, "content": [{ "type": "tool_use", "id": "t", "name": "get_project", "input": {} }], "usage": {} });
        let f = fake(vec![(200, vec![], refusal)]);
        let mut convo = Conversation { messages: vec![json!({ "role": "user", "content": "earlier" }), json!({ "role": "assistant", "content": [{ "type": "text", "text": "ok" }] })] };
        let mut t = recorder();
        let (r, _) = run(&cfg(&f), &mut convo, "Do something", &mut t);
        assert!(r.unwrap_err().contains("declined this request (not that)"));
        assert!(t.calls.is_empty(), "a refused turn's tools are never run");
        assert_eq!(convo.messages.len(), 2, "the conversation is as it was before the message");
    }

    #[test]
    fn a_failure_takes_the_whole_turn_back_out_even_after_tools_ran() {
        let f = fake(vec![
            (200, vec![], tool_use(vec![json!({ "type": "tool_use", "id": "toolu_1", "name": "get_project", "input": {} })])),
            (401, vec![], json!({ "type": "error", "error": { "type": "authentication_error", "message": "invalid x-api-key" } })),
        ]);
        let mut convo = Conversation::default();
        let mut t = recorder();
        let (r, _) = run(&cfg(&f), &mut convo, "Hi", &mut t);
        let e = r.unwrap_err();
        assert!(e.contains("API key was not accepted") && e.contains("invalid x-api-key"), "{e}");
        assert_eq!(t.calls.len(), 1, "the tool did run");
        assert!(convo.messages.is_empty(), "but the history holds no half-finished turn that the API would reject");
    }

    #[test]
    fn a_busy_api_is_retried_and_then_works() {
        let f = fake(vec![
            (529, vec![], json!({ "type": "error", "error": { "type": "overloaded_error", "message": "Overloaded" } })),
            (429, vec![("retry-after", "0")], json!({ "type": "error", "error": { "type": "rate_limit_error", "message": "slow down" } })),
            (200, vec![], end("Made it.")),
        ]);
        let mut convo = Conversation::default();
        let (r, events) = run(&cfg(&f), &mut convo, "Hi", &mut recorder());
        assert!(r.is_ok(), "{r:?}");
        assert!(events.contains(&Event::Text("Made it.".into())));
        assert_eq!(f.seen.lock().unwrap().len(), 3);
    }

    #[test]
    fn a_busy_api_that_stays_busy_gives_up_with_a_plain_message() {
        let busy = || (529, vec![], json!({ "type": "error", "error": { "type": "overloaded_error", "message": "Overloaded" } }));
        let f = fake(vec![busy(), busy(), busy(), busy()]);
        let (r, _) = run(&cfg(&f), &mut Conversation::default(), "Hi", &mut recorder());
        assert!(r.unwrap_err().contains("having trouble right now"));
        assert_eq!(f.seen.lock().unwrap().len(), 1 + MAX_RETRIES as usize);
    }

    #[test]
    fn an_optional_feature_the_api_does_not_take_is_dropped_and_the_request_tried_again() {
        let reject = |m: &str| (400, vec![], json!({ "type": "error", "error": { "type": "invalid_request_error", "message": m } }));
        let f = fake(vec![
            reject("Unexpected value(s) `server-side-fallback-2026-07-01` for the `anthropic-beta` header."),
            reject("cache_control: Extra inputs are not permitted"),
            reject("output_config.effort: not supported"),
            (200, vec![], end("Fine.")),
        ]);
        let (r, _) = run(&cfg(&f), &mut Conversation::default(), "Hi", &mut recorder());
        assert!(r.is_ok(), "{r:?}");
        let seen = f.seen.lock().unwrap();
        assert!(seen[1].0.iter().all(|(k, _)| k != "anthropic-beta") && seen[1].1.get("fallbacks").is_none(), "the fallback is dropped first");
        assert!(seen[2].1.get("cache_control").is_none() && seen[2].1.get("fallbacks").is_none());
        let last = &seen[3].1;
        assert!(last.get("output_config").is_none() && last.get("cache_control").is_none() && last.get("fallbacks").is_none(), "and finally a plain request that works");
        assert_eq!(last["messages"][0]["content"], "Hi");
    }

    #[test]
    fn a_bad_request_that_is_not_about_an_optional_feature_is_shown_as_it_is() {
        let f = fake(vec![(400, vec![], json!({ "type": "error", "error": { "type": "invalid_request_error", "message": "messages: roles must alternate" } }))]);
        let (r, _) = run(&cfg(&f), &mut Conversation::default(), "Hi", &mut recorder());
        assert!(r.unwrap_err().contains("roles must alternate"));
        assert_eq!(f.seen.lock().unwrap().len(), 1, "it is not retried");
    }

    #[test]
    fn the_cheap_model_does_not_ask_for_a_fallback_it_does_not_have() {
        let f = fake(vec![(200, vec![], end("Hi"))]);
        let mut c = cfg(&f);
        c.model = "claude-haiku-5-5".into();
        run(&c, &mut Conversation::default(), "Hi", &mut recorder()).0.unwrap();
        let (headers, body) = f.seen.lock().unwrap()[0].clone();
        assert!(body.get("fallbacks").is_none() && headers.iter().all(|(k, _)| k != "anthropic-beta"));
        assert_eq!(body["model"], "claude-haiku-5-5");
    }

    #[test]
    fn a_reply_cut_short_says_so_and_one_cut_in_a_step_is_an_error() {
        let cut = json!({ "role": "assistant", "stop_reason": "max_tokens", "content": [{ "type": "text", "text": "Part one" }], "usage": {} });
        let f = fake(vec![(200, vec![], cut)]);
        let (r, events) = run(&cfg(&f), &mut Conversation::default(), "Hi", &mut recorder());
        assert!(r.is_ok());
        assert!(events.iter().any(|e| matches!(e, Event::Notice(n) if n.contains("cut short"))));
        let mid = json!({ "role": "assistant", "stop_reason": "max_tokens", "content": [{ "type": "tool_use", "id": "t", "name": "get_project", "input": {} }], "usage": {} });
        let f = fake(vec![(200, vec![], mid)]);
        let mut t = recorder();
        let (r, _) = run(&cfg(&f), &mut Conversation::default(), "Hi", &mut t);
        assert!(r.unwrap_err().contains("cut off"));
        assert!(t.calls.is_empty(), "a half-written call is never run");
    }

    #[test]
    fn the_author_can_stop_it_and_a_missing_key_is_said_at_once() {
        let f = fake(vec![(200, vec![], end("never seen"))]);
        let stop = AtomicBool::new(true);
        let mut convo = Conversation::default();
        let r = run_turn(&cfg(&f), &mut convo, "Hi", &mut recorder(), &stop, &mut |_| {});
        assert_eq!(r.unwrap_err(), "Stopped.");
        assert!(convo.messages.is_empty() && f.seen.lock().unwrap().is_empty(), "nothing was sent and nothing kept");
        let mut c = cfg(&f);
        c.api_key = "  ".into();
        assert!(run(&c, &mut Conversation::default(), "Hi", &mut recorder()).0.unwrap_err().contains("no API key"));
    }

    #[test]
    fn a_loop_that_never_ends_stops_after_a_bounded_number_of_steps() {
        let again = || (200, vec![], tool_use(vec![json!({ "type": "tool_use", "id": "t", "name": "get_project", "input": {} })]));
        let f = fake((0..MAX_STEPS).map(|_| again()).collect());
        let mut t = recorder();
        let (r, events) = run(&cfg(&f), &mut Conversation::default(), "Loop", &mut t);
        assert!(r.is_ok());
        assert_eq!(t.calls.len(), MAX_STEPS);
        assert!(events.iter().any(|e| matches!(e, Event::Notice(n) if n.contains("Stopped after"))));
    }

    #[test]
    fn what_each_model_costs_is_known_for_the_listed_ones_and_a_typed_one_has_no_price() {
        for p in PROVIDERS {
            assert!(!p.models().is_empty());
            assert_eq!(default_model(p), p.models()[0].id);
            for m in p.models() {
                assert_eq!(find_model(p, m.id), Some(*m));
                let (i, o) = m.price.expect("every listed model has a price");
                assert!(i > 0.0 && o > i, "{}", m.id);
            }
            assert_eq!(find_model(p, "a-model-the-author-typed"), None);
            assert_eq!(Provider::from_id(p.id()), Some(p));
            assert_eq!(PROVIDERS[p.index()], p);
        }
        assert!(!find_model(Provider::Anthropic, "claude-haiku-5-5").unwrap().fallback);
    }

    fn cfg_for(f: &Fake, provider: Provider, model: &str) -> Config {
        let mut c = Config::new(provider, "key-123", model, "You help build lessons.");
        c.base_url = f.url.clone();
        c.retry_ms = 0;
        c
    }

    fn header(headers: &[(String, String)], k: &str) -> Option<String> { headers.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()) }

    fn openai_tools(content: &str, calls: Vec<Value>) -> Value {
        json!({ "choices": [{ "finish_reason": "tool_calls", "message": { "role": "assistant", "content": content, "refusal": null, "tool_calls": calls } }],
            "usage": { "prompt_tokens": 100, "completion_tokens": 20, "prompt_tokens_details": { "cached_tokens": 40 } } })
    }

    fn openai_says(text: &str) -> Value { json!({ "choices": [{ "finish_reason": "stop", "message": { "role": "assistant", "content": text } }], "usage": { "prompt_tokens": 10, "completion_tokens": 5 } }) }

    fn gpt_call(id: &str, args: &str) -> Value { json!({ "id": id, "type": "function", "function": { "name": "get_project", "arguments": args } }) }

    #[test]
    fn gpt_gets_the_tools_as_functions_and_its_results_come_back_one_message_each() {
        let f = fake(vec![
            (200, vec![], openai_tools("Let me look.", vec![gpt_call("call_1", r#"{"deep":true}"#), gpt_call("call_2", "")])),
            (200, vec![], openai_says("Two slides.")),
        ]);
        let mut convo = Conversation::default();
        let mut t = recorder();
        let (r, events) = run(&cfg_for(&f, Provider::OpenAi, "gpt-5"), &mut convo, "What is here?", &mut t);
        assert!(r.is_ok(), "{r:?}");
        assert_eq!(t.calls, vec![("get_project".to_string(), json!({ "deep": true })), ("get_project".to_string(), json!({}))], "arguments arrive as JSON text and an empty one is an empty object");
        assert!(events.contains(&Event::Usage { input: 60, output: 20, cache_read: 40, cache_write: 0 }), "cached tokens are counted apart from the rest of the prompt");
        assert!(events.contains(&Event::Text("Two slides.".into())));
        let seen = f.seen.lock().unwrap();
        let (headers, first) = &seen[0];
        assert_eq!((header(headers, ":path").as_deref(), header(headers, "authorization").as_deref()), (Some("/v1/chat/completions"), Some("Bearer key-123")));
        assert_eq!(first["model"], "gpt-5");
        assert_eq!(first["max_completion_tokens"], 16_000);
        assert_eq!(first["messages"][0], json!({ "role": "system", "content": "You help build lessons." }));
        assert_eq!(first["messages"][1], json!({ "role": "user", "content": "What is here?" }));
        assert_eq!((first["tools"][0]["type"].as_str(), first["tools"][0]["function"]["name"].as_str()), (Some("function"), Some("get_project")));
        assert!(first["tools"][0]["function"]["parameters"].is_object());
        let m = seen[1].1["messages"].as_array().unwrap().clone();
        assert_eq!(m.len(), 5, "system, question, the reply, and one tool message per call");
        assert_eq!(m[2]["tool_calls"].as_array().unwrap().len(), 2);
        assert_eq!(m[3], json!({ "role": "tool", "tool_call_id": "call_1", "content": "result of get_project" }));
        assert_eq!(m[4]["tool_call_id"], "call_2");
        assert_eq!(convo.messages.len(), 5, "the system prompt is sent each time, not kept");
    }

    #[test]
    fn gpt_arguments_that_are_not_json_are_told_back_and_the_tool_is_not_run() {
        let f = fake(vec![(200, vec![], openai_tools("", vec![gpt_call("call_1", "{not json")])), (200, vec![], openai_says("Sorry."))]);
        let mut t = recorder();
        let (r, events) = run(&cfg_for(&f, Provider::OpenAi, "gpt-5"), &mut Conversation::default(), "Go", &mut t);
        assert!(r.is_ok());
        assert!(t.calls.is_empty(), "a call that cannot be read is never run");
        assert!(events.iter().any(|e| matches!(e, Event::ToolEnd { ok: false, text, .. } if text.contains("could not be read"))));
        let back = f.seen.lock().unwrap()[1].1["messages"][3]["content"].as_str().unwrap().to_string();
        assert!(back.starts_with("Error: "), "{back}");
    }

    #[test]
    fn gpt_failures_and_refusals_are_plain_and_take_the_turn_back() {
        let f = fake(vec![(401, vec![], json!({ "error": { "message": "Incorrect API key provided", "type": "invalid_request_error" } }))]);
        let mut convo = Conversation::default();
        let e = run(&cfg_for(&f, Provider::OpenAi, "gpt-5"), &mut convo, "Hi", &mut recorder()).0.unwrap_err();
        assert!(e.contains("API key was not accepted") && e.contains("Incorrect API key"), "{e}");
        assert!(convo.messages.is_empty());
        let refusal = json!({ "choices": [{ "finish_reason": "stop", "message": { "role": "assistant", "content": null, "refusal": "I can't help with that." } }], "usage": {} });
        let f = fake(vec![(200, vec![], refusal)]);
        let e = run(&cfg_for(&f, Provider::OpenAi, "gpt-5"), &mut convo, "Hi", &mut recorder()).0.unwrap_err();
        assert!(e.contains("GPT declined this request (I can't help with that.)"), "{e}");
        assert!(convo.messages.is_empty());
        let f = fake(vec![(404, vec![], json!({ "error": { "message": "The model  does not exist" } }))]);
        let e = run(&cfg_for(&f, Provider::OpenAi, "gpt-9"), &mut Conversation::default(), "Hi", &mut recorder()).0.unwrap_err();
        assert!(e.contains("not available to this API key") && e.contains("type a model id"), "{e}");
    }

    #[test]
    fn gpt_is_never_sent_claudes_optional_features() {
        let f = fake(vec![(400, vec![], json!({ "error": { "message": "Unknown parameter: cache_control" } }))]);
        let e = run(&cfg_for(&f, Provider::OpenAi, "gpt-5"), &mut Conversation::default(), "Hi", &mut recorder()).0.unwrap_err();
        assert!(e.contains("cache_control"), "a 400 is shown as it is, not retried");
        assert_eq!(f.seen.lock().unwrap().len(), 1);
        let body = &f.seen.lock().unwrap()[0].1;
        assert!(body.get("cache_control").is_none() && body.get("fallbacks").is_none() && body.get("output_config").is_none());
    }

    fn gemini_reply(parts: Vec<Value>, finish: &str) -> Value {
        json!({ "candidates": [{ "content": { "role": "model", "parts": parts }, "finishReason": finish }],
            "usageMetadata": { "promptTokenCount": 100, "cachedContentTokenCount": 30, "candidatesTokenCount": 20, "thoughtsTokenCount": 15 } })
    }

    #[test]
    fn gemini_gets_function_declarations_keeps_its_signatures_and_all_results_in_one_message() {
        let signed = json!({ "functionCall": { "name": "get_project", "args": { "deep": true } }, "thoughtSignature": "sig-abc" });
        let f = fake(vec![
            (200, vec![], gemini_reply(vec![json!({ "text": "pondering", "thought": true }), json!({ "text": "Let me look." }), signed.clone(), json!({ "functionCall": { "name": "get_project", "id": "c2" } })], "STOP")),
            (200, vec![], gemini_reply(vec![json!({ "text": "Two slides." })], "STOP")),
        ]);
        let mut convo = Conversation::default();
        let mut t = recorder();
        let (r, events) = run(&cfg_for(&f, Provider::Gemini, "gemini-3-flash-preview"), &mut convo, "What is here?", &mut t);
        assert!(r.is_ok(), "{r:?}");
        assert_eq!(t.calls, vec![("get_project".to_string(), json!({ "deep": true })), ("get_project".to_string(), json!({}))]);
        assert!(events.contains(&Event::Usage { input: 70, output: 35, cache_read: 30, cache_write: 0 }), "thinking counts as output");
        assert!(events.contains(&Event::Text("Let me look.".into())) && !events.contains(&Event::Text("pondering".into())), "a thought summary is not shown");
        let seen = f.seen.lock().unwrap();
        let (headers, first) = &seen[0];
        assert_eq!((header(headers, ":path").as_deref(), header(headers, "x-goog-api-key").as_deref()), (Some("/v1beta/models/gemini-3-flash-preview:generateContent"), Some("key-123")));
        assert_eq!(first["systemInstruction"]["parts"][0]["text"], "You help build lessons.");
        assert_eq!(first["contents"][0], json!({ "role": "user", "parts": [{ "text": "What is here?" }] }));
        assert_eq!(first["tools"][0]["functionDeclarations"][0]["name"], "get_project");
        assert!(first["tools"][0]["functionDeclarations"][0]["parametersJsonSchema"].is_object());
        assert_eq!(first["generationConfig"]["maxOutputTokens"], 16_000);
        let contents = seen[1].1["contents"].as_array().unwrap().clone();
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[1]["parts"][2], signed, "the thought signature goes back in the same part");
        let sent_back = contents[2]["parts"].as_array().unwrap();
        assert_eq!((contents[2]["role"].as_str(), sent_back.len()), (Some("user"), 2), "both results in one message");
        assert_eq!(sent_back[0]["functionResponse"], json!({ "name": "get_project", "response": { "result": "result of get_project" } }));
        assert_eq!(sent_back[1]["functionResponse"]["id"], "c2", "an id the model gave is echoed");
    }

    #[test]
    fn gemini_blocks_and_errors_are_plain_and_take_the_turn_back() {
        let mut convo = Conversation::default();
        for (reply, expect) in [
            (json!({ "promptFeedback": { "blockReason": "PROHIBITED_CONTENT" }, "usageMetadata": {} }), "Gemini declined this request (PROHIBITED_CONTENT)"),
            (gemini_reply(vec![], "SAFETY"), "Gemini declined this request (SAFETY)"),
            (gemini_reply(vec![], "MALFORMED_FUNCTION_CALL"), "could not be read"),
        ] {
            let f = fake(vec![(200, vec![], reply)]);
            let e = run(&cfg_for(&f, Provider::Gemini, "gemini-3-flash-preview"), &mut convo, "Hi", &mut recorder()).0.unwrap_err();
            assert!(e.contains(expect), "{e}");
            assert!(convo.messages.is_empty());
        }
        let f = fake(vec![(403, vec![], json!({ "error": { "code": 403, "message": "API key not valid", "status": "PERMISSION_DENIED" } }))]);
        let e = run(&cfg_for(&f, Provider::Gemini, "gemini-3-flash-preview"), &mut convo, "Hi", &mut recorder()).0.unwrap_err();
        assert!(e.contains("not allowed") && e.contains("API key not valid"), "{e}");
        let f = fake(vec![(200, vec![], gemini_reply(vec![json!({ "text": "Part one" })], "MAX_TOKENS"))]);
        let (r, events) = run(&cfg_for(&f, Provider::Gemini, "gemini-3-flash-preview"), &mut convo, "Hi", &mut recorder());
        assert!(r.is_ok() && events.iter().any(|e| matches!(e, Event::Notice(n) if n.contains("cut short"))));
    }

    #[test]
    fn an_empty_gemini_reply_is_not_kept_because_the_api_rejects_a_message_with_no_parts() {
        let f = fake(vec![(200, vec![], gemini_reply(vec![], "STOP"))]);
        let mut convo = Conversation::default();
        assert!(run(&cfg_for(&f, Provider::Gemini, "gemini-3-flash-preview"), &mut convo, "Hi", &mut recorder()).0.is_ok());
        assert_eq!(convo.messages.len(), 1, "only the question");
    }

    #[test]
    fn each_provider_has_somewhere_to_get_a_key_and_a_name() {
        for p in PROVIDERS {
            assert!(p.key_url().starts_with("https://") && p.key_url().contains(p.billing_site()), "{}", p.id());
            assert!(!p.name().is_empty() && p.label().contains(p.company()) && !p.env_keys().is_empty() && !p.key_hint().is_empty());
        }
        assert_eq!(Provider::Gemini.env_keys(), ["GEMINI_API_KEY", "GOOGLE_API_KEY"]);
    }
}
