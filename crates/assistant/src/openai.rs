//! The OpenAI Chat Completions API.
use super::*;

pub(crate) const MODELS: [Model; 3] = [
    Model { id: "gpt-5", name: "GPT-5", note: "Balanced", price: Some((1.25, 10.0)), fallback: false },
    Model { id: "gpt-5-mini", name: "GPT-5 mini", note: "Faster and cheaper", price: Some((0.25, 2.0)), fallback: false },
    Model { id: "gpt-5.5", name: "GPT-5.5", note: "Newest and most capable", price: Some((5.0, 30.0)), fallback: false },
];

pub(crate) fn request(cfg: &Config, defs: &[Value], messages: &[Value]) -> Request {
    let tools: Vec<Value> = defs.iter().map(|d| json!({ "type": "function", "function": { "name": d["name"], "description": d["description"], "parameters": d["input_schema"] } })).collect();
    let mut all = vec![json!({ "role": "system", "content": cfg.system })];
    all.extend(messages.iter().cloned());
    let body = json!({ "model": cfg.model, "max_completion_tokens": cfg.max_tokens, "messages": all, "tools": tools });
    Request { url: format!("{}/v1/chat/completions", cfg.base_url.trim_end_matches('/')), headers: vec![("authorization", format!("Bearer {}", cfg.api_key.trim()))], body }
}

pub(crate) fn read(reply: &Value) -> Step {
    let usage = &reply["usage"];
    let cached = n(&usage["prompt_tokens_details"]["cached_tokens"]);
    let tokens = Tokens { input: n(&usage["prompt_tokens"]).saturating_sub(cached), output: n(&usage["completion_tokens"]), cache_read: cached, cache_write: 0 };
    let choice = &reply["choices"][0];
    let msg = &choice["message"];
    if let Some(r) = msg["refusal"].as_str().filter(|r| !r.is_empty()) {
        return Step { message: None, texts: vec![], calls: vec![], tokens, end: End::Refused(r.to_string()) };
    }
    let finish = choice["finish_reason"].as_str().unwrap_or("stop");
    if finish == "content_filter" { return Step { message: None, texts: vec![], calls: vec![], tokens, end: End::Refused("it was stopped by a content filter".into()) }; }
    if msg.is_null() { return Step { message: None, texts: vec![], calls: vec![], tokens, end: End::Bad("GPT's reply had no answer in it.".into()) }; }
    let raw = msg["tool_calls"].as_array().cloned().unwrap_or_default();
    let calls: Vec<Call> = raw.iter().map(|c| {
        let args = c["function"]["arguments"].as_str().unwrap_or("");
        Call {
            id: c["id"].as_str().unwrap_or_default().to_string(),
            name: c["function"]["name"].as_str().unwrap_or_default().to_string(),
            // A call with no arguments at all is an empty object.
            input: if args.trim().is_empty() { Ok(json!({})) } else { serde_json::from_str(args).map_err(|e| e.to_string()) },
        }
    }).collect();
    let text = msg["content"].as_str().unwrap_or("").to_string();
    let mut kept = json!({ "role": "assistant", "content": text });
    if !raw.is_empty() { kept["tool_calls"] = json!(raw); }
    let end = match finish { "length" => End::Cut, _ if !calls.is_empty() => End::Tools, _ => End::Done };
    Step { message: Some(kept), texts: vec![text], calls, tokens, end }
}

/// Each result is a message of its own, in the order the calls came.
pub(crate) fn results(outcomes: &[Outcome]) -> Vec<Value> {
    outcomes.iter().map(|o| json!({ "role": "tool", "tool_call_id": o.id, "content": if o.ok { o.text.clone() } else { format!("Error: {}", o.text) } })).collect()
}
