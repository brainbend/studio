//! The Gemini `generateContent` API.
use super::*;

pub(crate) const MODELS: [Model; 2] = [
    Model { id: "gemini-3.1-pro-preview", name: "Gemini 3.1 Pro", note: "Most capable", price: Some((2.0, 12.0)), fallback: false },
    Model { id: "gemini-3-flash-preview", name: "Gemini 3 Flash", note: "Faster and cheaper", price: Some((0.5, 3.0)), fallback: false },
];

pub(crate) fn request(cfg: &Config, defs: &[Value], messages: &[Value]) -> Request {
    // `parametersJsonSchema` takes the tools' JSON Schemas as they are; the older `parameters` field takes only a subset.
    let declarations: Vec<Value> = defs.iter().map(|d| json!({ "name": d["name"], "description": d["description"], "parametersJsonSchema": d["input_schema"] })).collect();
    let body = json!({
        "systemInstruction": { "parts": [{ "text": cfg.system }] },
        "contents": messages,
        "tools": [{ "functionDeclarations": declarations }],
        "generationConfig": { "maxOutputTokens": cfg.max_tokens },
    });
    let url = format!("{}/v1beta/models/{}:generateContent", cfg.base_url.trim_end_matches('/'), cfg.model);
    Request { url, headers: vec![("x-goog-api-key", cfg.api_key.trim().to_string())], body }
}

pub(crate) fn read(reply: &Value) -> Step {
    let usage = &reply["usageMetadata"];
    let cached = n(&usage["cachedContentTokenCount"]);
    // Thinking is billed as output.
    let tokens = Tokens { input: n(&usage["promptTokenCount"]).saturating_sub(cached), output: n(&usage["candidatesTokenCount"]) + n(&usage["thoughtsTokenCount"]), cache_read: cached, cache_write: 0 };
    let refused = |why: &str, tokens| Step { message: None, texts: vec![], calls: vec![], tokens, end: End::Refused(why.to_string()) };
    let cand = &reply["candidates"][0];
    if cand.is_null() {
        let why = reply["promptFeedback"]["blockReason"].as_str().unwrap_or("it gave no answer");
        return refused(why, tokens);
    }
    let finish = cand["finishReason"].as_str().unwrap_or("STOP");
    if matches!(finish, "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" | "IMAGE_SAFETY") { return refused(finish, tokens); }
    if matches!(finish, "MALFORMED_FUNCTION_CALL" | "UNEXPECTED_TOOL_CALL") {
        return Step { message: None, texts: vec![], calls: vec![], tokens, end: End::Bad("Gemini wrote a tool call that could not be read. Try again.".into()) };
    }
    let parts = cand["content"]["parts"].as_array().cloned().unwrap_or_default();
    // Thought summaries are parts too; they stay in the history but are not shown.
    let texts = parts.iter().filter(|p| p["thought"] != true).filter_map(|p| p["text"].as_str().map(str::to_string)).collect();
    let calls: Vec<Call> = parts.iter().filter(|p| p.get("functionCall").is_some()).map(|p| Call {
        id: p["functionCall"]["id"].as_str().unwrap_or_default().to_string(),
        name: p["functionCall"]["name"].as_str().unwrap_or_default().to_string(),
        input: Ok(if p["functionCall"]["args"].is_null() { json!({}) } else { p["functionCall"]["args"].clone() }),
    }).collect();
    // Kept exactly as it came: Gemini 3 wants its thought signatures back in the same parts. A reply with nothing in it is
    // not kept, since the API rejects a message with no parts.
    let message = if parts.is_empty() { None } else { Some(json!({ "role": "model", "parts": parts })) };
    let end = match finish { "MAX_TOKENS" => End::Cut, _ if !calls.is_empty() => End::Tools, _ => End::Done };
    Step { message, texts, calls, tokens, end }
}

/// All of a turn's results go back together, as the parts of one message.
pub(crate) fn results(outcomes: &[Outcome]) -> Vec<Value> {
    let parts: Vec<Value> = outcomes.iter().map(|o| {
        let mut response = json!({ "name": o.name, "response": if o.ok { json!({ "result": o.text }) } else { json!({ "error": o.text }) } });
        if !o.id.is_empty() { response["id"] = json!(o.id); }
        json!({ "functionResponse": response })
    }).collect();
    vec![json!({ "role": "user", "parts": parts })]
}
