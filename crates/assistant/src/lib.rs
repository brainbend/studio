//! The assistant built into BrainBend Studio: a chat in which Claude uses the app's tools to build and change a lesson.
//!
//! This crate knows nothing about slides. It speaks the Claude Messages API over HTTP (there is no official Rust SDK, so
//! it is plain requests) and runs the tool-use loop: send the conversation and the tools, run whichever tools Claude asks
//! for, send their results back, and repeat until Claude has finished. The app supplies the tools ([`Tools`]) and shows
//! the events ([`Event`]).
//!
//! Things the API needs that are easy to get wrong, and are done here:
//! - Claude's reply is kept in the history exactly as it came, thinking blocks included, and every result of a turn's tool
//!   calls goes back in one message.
//! - A refusal is handled before anything in the reply is read, and a turn that fails is taken back out of the history, so
//!   one error cannot leave the conversation in a shape the API rejects.
//! - Optional request features (a beta fallback model, prompt caching, effort) are dropped, and the request tried again, if
//!   the API says it does not take them, so a feature this build guessed wrong never stops the chat from working.
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const API_VERSION: &str = "2026-10-21";
/// The beta that turns on `fallbacks: "default"`: a request a safety classifier declines is re-run on a fallback model.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// The most requests one message from the author may cause, so a loop that never ends costs a bounded amount.
const MAX_STEPS: usize = 40;
const MAX_RETRIES: u32 = 3;

/// The models the author can choose, with what they cost per million tokens (input, output) and whether the API can re-run
/// a declined request on a fallback model for them.
pub const MODELS: [Model; 3] = [
    Model { id: "claude-opus-5-5", name: "Claude Opus 5.5", note: "Most capable", input: 4.0, output: 20.0, fallback: true },
    Model { id: "claude-sonnet-5-5", name: "Claude Sonnet 5.5", note: "Faster and cheaper", input: 2.0, output: 10.0, fallback: true },
    Model { id: "claude-haiku-5-5", name: "Claude Haiku 5.5", note: "Fastest and cheapest", input: 0.10, output: 0.50, fallback: false },
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub note: &'static str,
    /// US dollars per million input tokens, and per million output tokens.
    pub input: f64,
    pub output: f64,
    pub fallback: bool,
}

pub fn model(id: &str) -> Model { MODELS.iter().copied().find(|m| m.id == id).unwrap_or(MODELS[0]) }

/// How to reach Claude.
#[derive(Clone, Debug)]
pub struct Config {
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
    pub fn new(api_key: &str, model: &str, system: &str) -> Self {
        Self { api_key: api_key.trim().to_string(), model: model.to_string(), base_url: "https://api.anthropic.com".into(), max_tokens: 16_000, retry_ms: 1000, system: system.to_string() }
    }
}

/// What the app shows as a turn goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Words from Claude.
    Text(String),
    /// Claude is using a tool.
    ToolStart { name: String, input: Value },
    /// The tool finished: whether it worked, and what it said.
    ToolEnd { name: String, ok: bool, text: String },
    /// Something the author should know that is not an error.
    Notice(String),
    /// Tokens used by one request.
    Usage { input: u64, output: u64, cache_read: u64, cache_write: u64 },
}

/// What Claude can do for the app: the tools, and running one.
pub trait Tools {
    /// Each tool as the API wants it: `name`, `description` and `input_schema`.
    fn definitions(&self) -> Vec<Value>;
    /// Runs a tool: what it says, or why it could not.
    fn call(&mut self, name: &str, input: &Value) -> Result<String, String>;
}

/// The conversation so far, in the form the API takes.
#[derive(Clone, Debug, Default)]
pub struct Conversation {
    pub messages: Vec<Value>,
}

/// Optional parts of a request, each dropped if the API does not take it.
#[derive(Clone, Copy)]
struct Features { fallback: bool, cache: bool, effort: bool }

/// The text of an error reply, or what the status means.
fn explain(status: u16, body: &str) -> String {
    let api = serde_json::from_str::<Value>(body).ok().and_then(|v| v["error"]["message"].as_str().map(str::to_string));
    let base = match status {
        401 => "The API key was not accepted. Check it in the AI tab.".to_string(),
        402 => "Your Claude account has a billing problem. Check it at console.anthropic.com.".to_string(),
        403 => "This API key is not allowed to do that.".to_string(),
        404 => "That model is not available to this API key.".to_string(),
        413 => "The conversation is too big to send. Start a new chat.".to_string(),
        429 => "Claude is busy or this key has hit its rate limit. Wait a moment and try again.".to_string(),
        500..=599 => "Claude is having trouble right now. Try again in a moment.".to_string(),
        _ => "Claude could not take the request.".to_string(),
    };
    match api { Some(m) if !m.is_empty() => format!("{base} ({m})"), _ => base }
}

fn retryable(status: u16) -> bool { matches!(status, 429 | 500 | 502 | 503 | 504 | 529) }

/// One request, retried when the API is busy. Returns the reply, or why it failed (with the status for a 4xx).
fn post(cfg: &Config, body: &Value, features: Features, cancel: &AtomicBool) -> Result<Value, (Option<u16>, String)> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(600))).http_status_as_error(false).build().into();
    let url = format!("{}/v1/messages", cfg.base_url.trim_end_matches('/'));
    let mut tries = 0;
    loop {
        if cancel.load(Ordering::Relaxed) { return Err((None, "Stopped.".into())); }
        let mut req = agent.post(&url).header("x-api-key", cfg.api_key.trim()).header("anthropic-version", API_VERSION).header("content-type", "application/json");
        if features.fallback { req = req.header("anthropic-beta", FALLBACK_BETA); }
        let sent = req.send(body.to_string().as_bytes());
        let (status, text, wait) = match sent {
            Ok(mut reply) => {
                let status = reply.status().as_u16();
                let wait = reply.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<u64>().ok());
                (status, reply.body_mut().read_to_string().unwrap_or_default(), wait)
            }
            Err(e) => (0, e.to_string(), None),
        };
        if status == 200 { return serde_json::from_str(&text).map_err(|e| (None, format!("Claude's reply could not be read: {e}"))); }
        let transport = status == 0;
        if (transport || retryable(status)) && tries < MAX_RETRIES {
            tries += 1;
            let ms = wait.map(|s| (s * 1000).min(20_000)).unwrap_or(cfg.retry_ms << (tries - 1));
            let mut left = ms;
            while left > 0 && !cancel.load(Ordering::Relaxed) { let step = left.min(100); std::thread::sleep(Duration::from_millis(step)); left -= step; }
            continue;
        }
        return Err(if transport { (None, "Could not reach Claude. Check the internet connection.".into()) } else { (Some(status), explain(status, &text)) });
    }
}

/// Which optional feature a 400 is about, if it names one.
fn rejected(message: &str, features: Features) -> Option<Features> {
    let m = message.to_ascii_lowercase();
    if features.fallback && (m.contains("fallback") || m.contains("anthropic-beta")) { return Some(Features { fallback: false, ..features }); }
    if features.cache && m.contains("cache_control") { return Some(Features { cache: false, ..features }); }
    if features.effort && (m.contains("effort") || m.contains("output_config")) { return Some(Features { effort: false, ..features }); }
    None
}

fn body(cfg: &Config, tools: &[Value], messages: &[Value], features: Features) -> Value {
    let mut v = json!({ "model": cfg.model, "max_tokens": cfg.max_tokens, "system": cfg.system, "tools": tools, "messages": messages });
    if features.effort { v["output_config"] = json!({ "effort": "medium" }); }
    if features.cache { v["cache_control"] = json!({ "type": "ephemeral" }); }
    if features.fallback { v["fallbacks"] = json!("default"); }
    v
}

/// Runs one message from the author to the end: asks Claude, runs the tools it asks for, and repeats. `emit` gets each
/// event as it happens. On any failure the turn is taken back out of the conversation, so the next message starts clean.
pub fn run_turn(cfg: &Config, convo: &mut Conversation, text: &str, tools: &mut dyn Tools, cancel: &AtomicBool, emit: &mut dyn FnMut(Event)) -> Result<(), String> {
    let start = convo.messages.len();
    let result = turn(cfg, convo, text, tools, cancel, emit);
    if result.is_err() { convo.messages.truncate(start); }
    result
}

fn turn(cfg: &Config, convo: &mut Conversation, text: &str, tools: &mut dyn Tools, cancel: &AtomicBool, emit: &mut dyn FnMut(Event)) -> Result<(), String> {
    if cfg.api_key.trim().is_empty() { return Err("There is no API key. Add one in the AI tab.".into()); }
    let defs = tools.definitions();
    let mut features = Features { fallback: model(&cfg.model).fallback, cache: true, effort: true };
    convo.messages.push(json!({ "role": "user", "content": text }));
    for _ in 0..MAX_STEPS {
        let reply = loop {
            match post(cfg, &body(cfg, &defs, &convo.messages, features), features, cancel) {
                Ok(r) => break r,
                Err((Some(400), message)) => match rejected(&message, features) { Some(f) => features = f, None => return Err(message) },
                Err((_, message)) => return Err(message),
            }
        };
        let usage = &reply["usage"];
        let n = |k: &str| usage[k].as_u64().unwrap_or(0);
        emit(Event::Usage { input: n("input_tokens"), output: n("output_tokens"), cache_read: n("cache_read_input_tokens"), cache_write: n("cache_creation_input_tokens") });
        let stop = reply["stop_reason"].as_str().unwrap_or("end_turn").to_string();
        // A refusal is read first: nothing in a refused reply is to be run or kept.
        if stop == "refusal" {
            let why = reply["stop_details"]["explanation"].as_str().filter(|e| !e.is_empty()).map(|e| format!(" ({e})")).unwrap_or_default();
            return Err(format!("Claude declined this request{why}. Try asking for it another way."));
        }
        let content = reply["content"].as_array().cloned().unwrap_or_default();
        // Kept exactly as it came: the thinking blocks in it must go back unchanged.
        convo.messages.push(json!({ "role": "assistant", "content": content }));
        for block in &content {
            if block["type"] == "text" { if let Some(t) = block["text"].as_str().filter(|t| !t.trim().is_empty()) { emit(Event::Text(t.to_string())); } }
        }
        let calls: Vec<&Value> = content.iter().filter(|b| b["type"] == "tool_use").collect();
        match stop.as_str() {
            "tool_use" if !calls.is_empty() => {
                let mut results = vec![];
                for call in calls {
                    let (id, name, input) = (call["id"].as_str().unwrap_or_default().to_string(), call["name"].as_str().unwrap_or_default().to_string(), call["input"].clone());
                    if cancel.load(Ordering::Relaxed) { return Err("Stopped.".into()); }
                    emit(Event::ToolStart { name: name.clone(), input: input.clone() });
                    let outcome = tools.call(&name, &input);
                    let (ok, out) = match outcome { Ok(t) => (true, t), Err(e) => (false, e) };
                    emit(Event::ToolEnd { name: name.clone(), ok, text: out.clone() });
                    let mut block = json!({ "type": "tool_result", "tool_use_id": id, "content": out });
                    if !ok { block["is_error"] = json!(true); }
                    results.push(block);
                }
                // Every result of this turn's calls goes back in one message.
                convo.messages.push(json!({ "role": "user", "content": results }));
            }
            "max_tokens" => {
                if !calls.is_empty() { return Err("Claude's reply was cut off in the middle of a step. Try a smaller request.".into()); }
                emit(Event::Notice("The reply was cut short. Say \"continue\" to go on.".into()));
                return Ok(());
            }
            "model_context_window_exceeded" => return Err("The conversation is too long for Claude. Start a new chat.".into()),
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
                let sent: Vec<(String, String)> = req.headers().iter().map(|h| (h.field.as_str().as_str().to_ascii_lowercase(), h.value.as_str().to_string())).collect();
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
        let mut c = Config::new("sk-test", "claude-opus-5-5", "You help build lessons.");
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
        assert_eq!((header("x-api-key").as_deref(), header("anthropic-version").as_deref(), header("anthropic-beta").as_deref()), (Some("sk-test"), Some("2026-10-21"), Some(FALLBACK_BETA)));
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
    fn what_each_model_costs_is_known() {
        assert_eq!(model("claude-opus-5-5").id, "claude-opus-5-5");
        assert_eq!(model("nonsense").id, "claude-opus-5-5", "an unknown model is the default");
        assert!(MODELS.iter().all(|m| m.input > 0.0 && m.output > m.input));
        assert!(!model("claude-haiku-5-5").fallback);
    }
}
