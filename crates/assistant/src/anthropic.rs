//! The Claude Messages API.
use super::*;

const API_VERSION: &str = "2026-10-21";
/// The beta that turns on `fallbacks: "default"`: a request a safety classifier declines is re-run on a fallback model.
pub(crate) const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

pub(crate) const MODELS: [Model; 3] = [
    Model { id: "claude-opus-5-5", name: "Claude Opus 5.5", note: "Most capable", price: Some((4.0, 20.0)), fallback: true },
    Model { id: "claude-sonnet-5-5", name: "Claude Sonnet 5.5", note: "Faster and cheaper", price: Some((2.0, 10.0)), fallback: true },
    Model { id: "claude-haiku-5-5", name: "Claude Haiku 5.5", note: "Fastest and cheapest", price: Some((0.10, 0.50)), fallback: false },
];

pub(crate) fn request(cfg: &Config, defs: &[Value], messages: &[Value], features: Features) -> Request {
    let mut body = json!({ "model": cfg.model, "max_tokens": cfg.max_tokens, "system": cfg.system, "tools": defs, "messages": messages });
    if features.effort { body["output_config"] = json!({ "effort": "medium" }); }
    if features.cache { body["cache_control"] = json!({ "type": "ephemeral" }); }
    if features.fallback { body["fallbacks"] = json!("default"); }
    let mut headers = vec![("x-api-key", cfg.api_key.trim().to_string()), ("anthropic-version", API_VERSION.to_string())];
    if features.fallback { headers.push(("anthropic-beta", FALLBACK_BETA.to_string())); }
    Request { url: format!("{}/v1/messages", cfg.base_url.trim_end_matches('/')), headers, body }
}

pub(crate) fn read(reply: &Value) -> Step {
    let usage = &reply["usage"];
    let tokens = Tokens { input: n(&usage["input_tokens"]), output: n(&usage["output_tokens"]), cache_read: n(&usage["cache_read_input_tokens"]), cache_write: n(&usage["cache_creation_input_tokens"]) };
    let stop = reply["stop_reason"].as_str().unwrap_or("end_turn");
    if stop == "refusal" {
        let why = reply["stop_details"]["explanation"].as_str().unwrap_or("").to_string();
        return Step { message: None, texts: vec![], calls: vec![], tokens, end: End::Refused(why) };
    }
    let content = reply["content"].as_array().cloned().unwrap_or_default();
    let texts = content.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str().map(str::to_string)).collect();
    let calls: Vec<Call> = content.iter().filter(|b| b["type"] == "tool_use").map(|b| Call {
        id: b["id"].as_str().unwrap_or_default().to_string(),
        name: b["name"].as_str().unwrap_or_default().to_string(),
        input: Ok(b["input"].clone()),
    }).collect();
    let end = match stop {
        "tool_use" if !calls.is_empty() => End::Tools,
        "max_tokens" => End::Cut,
        "model_context_window_exceeded" => End::TooLong,
        _ => End::Done,
    };
    Step { message: Some(json!({ "role": "assistant", "content": content })), texts, calls, tokens, end }
}

pub(crate) fn results(outcomes: &[Outcome]) -> Vec<Value> {
    let blocks: Vec<Value> = outcomes.iter().map(|o| {
        let mut block = json!({ "type": "tool_result", "tool_use_id": o.id, "content": o.text });
        if !o.ok { block["is_error"] = json!(true); }
        block
    }).collect();
    vec![json!({ "role": "user", "content": blocks })]
}
