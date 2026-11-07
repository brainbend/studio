//! The Model Context Protocol, the part that does not need a network: reading a JSON-RPC message and deciding what to
//! answer. MCP is JSON-RPC 2.0; the controller offers only tools, so it answers `initialize`, `ping`, `tools/list` and
//! `tools/call`, and takes notifications without answering.
use serde_json::{json, Value};

/// The protocol versions this controller can speak, newest first.
pub const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// A tool the controller offers.
#[derive(Clone, Debug)]
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    /// A JSON Schema for the arguments (an object).
    pub schema: Value,
    /// The tool changes nothing (a hint for the AI tool, which may skip asking before it calls it).
    pub read_only: bool,
}

/// What to do about a message.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Answer with this JSON.
    Respond(Value),
    /// Nothing to answer: a notification, or a reply to something we never asked. The HTTP answer is 202.
    Accepted,
    /// Run a tool, then answer with [`tool_result`].
    Call { id: Value, name: String, arguments: Value },
}

pub fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// The answer to a tool call: its text, or an error the AI tool is meant to read and react to (a tool that fails is a
/// result with `isError`, not a protocol error).
pub fn tool_result(id: Value, result: Result<String, String>) -> Value {
    let (text, is_error) = match result { Ok(t) => (t, false), Err(e) => (e, true) };
    json!({ "jsonrpc": "2.0", "id": id, "result": { "content": [{ "type": "text", "text": text }], "isError": is_error } })
}

fn ok(id: Value, result: Value) -> Value { json!({ "jsonrpc": "2.0", "id": id, "result": result }) }

/// Reads one message `body` and says what to do about it.
pub fn handle(body: &[u8], tools: &[ToolDef], version: &str, instructions: &str) -> Action {
    let msg: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return Action::Respond(error(Value::Null, -32700, "the message is not valid JSON")),
    };
    // MCP over HTTP sends one message at a time (the 2025-06-18 protocol dropped batches).
    let Some(obj) = msg.as_object() else { return Action::Respond(error(Value::Null, -32600, "expected one JSON-RPC message, not a list")) };
    let id = obj.get("id").cloned();
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        // A reply from the client (it has a result or error and no method): nothing to do.
        return if obj.contains_key("result") || obj.contains_key("error") { Action::Accepted } else { Action::Respond(error(id.unwrap_or(Value::Null), -32600, "the message has no method")) };
    };
    // A message without an id is a notification: it gets no answer, whatever it says.
    let Some(id) = id else { return Action::Accepted };
    let params = obj.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
            let chosen = VERSIONS.iter().find(|v| **v == asked).unwrap_or(&VERSIONS[0]);
            Action::Respond(ok(id, json!({
                "protocolVersion": chosen,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "brainbend-studio", "version": version },
                "instructions": instructions,
            })))
        }
        "ping" => Action::Respond(ok(id, json!({}))),
        "tools/list" => Action::Respond(ok(id, json!({
            "tools": tools.iter().map(|t| json!({
                "name": t.name, "description": t.description, "inputSchema": t.schema,
                "annotations": { "readOnlyHint": t.read_only },
            })).collect::<Vec<_>>(),
        }))),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else { return Action::Respond(error(id, -32602, "tools/call needs a tool name")) };
            if !tools.iter().any(|t| t.name == name) { return Action::Respond(error(id, -32602, &format!("there is no tool called {name}"))); }
            let arguments = params.get("arguments").cloned().filter(|a| !a.is_null()).unwrap_or_else(|| json!({}));
            if !arguments.is_object() { return Action::Respond(error(id, -32602, "the arguments must be an object")); }
            Action::Call { id, name: name.to_string(), arguments }
        }
        other => Action::Respond(error(id, -32601, &format!("this controller does not offer {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> Vec<ToolDef> {
        vec![
            ToolDef { name: "get_project", description: "Facts about the open project", schema: json!({ "type": "object", "properties": {} }), read_only: true },
            ToolDef { name: "add_slide", description: "Add a slide", schema: json!({ "type": "object" }), read_only: false },
        ]
    }

    fn run(body: &str) -> Action { handle(body.as_bytes(), &tools(), "0.3.0", "Be careful.") }

    fn responded(a: Action) -> Value { match a { Action::Respond(v) => v, other => panic!("expected an answer, got {other:?}") } }

    #[test]
    fn initialize_agrees_a_protocol_version_and_says_what_the_controller_offers() {
        let v = responded(run(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"x","version":"1"}}}"#));
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["protocolVersion"], "2025-03-26", "a version it speaks is kept");
        assert_eq!(v["result"]["serverInfo"], json!({ "name": "brainbend-studio", "version": "0.3.0" }));
        assert_eq!(v["result"]["capabilities"], json!({ "tools": { "listChanged": false } }));
        assert_eq!(v["result"]["instructions"], "Be careful.");
        let newer = responded(run(r#"{"jsonrpc":"2.0","id":"a","method":"initialize","params":{"protocolVersion":"2099-01-01"}}"#));
        assert_eq!(newer["result"]["protocolVersion"], "2025-06-18", "one it does not know gets the newest it has");
        assert_eq!(newer["id"], "a", "the id comes back as it was sent");
    }

    #[test]
    fn notifications_and_replies_get_no_answer() {
        assert_eq!(run(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#), Action::Accepted);
        assert_eq!(run(r#"{"jsonrpc":"2.0","method":"tools/list"}"#), Action::Accepted, "without an id it is a notification, whatever it asks");
        assert_eq!(run(r#"{"jsonrpc":"2.0","id":7,"result":{}}"#), Action::Accepted);
    }

    #[test]
    fn tools_are_listed_with_their_schemas_and_whether_they_only_read() {
        let v = responded(run(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#));
        let list = v["result"]["tools"].as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["name"], "get_project");
        assert_eq!(list[0]["inputSchema"]["type"], "object");
        assert_eq!(list[0]["annotations"]["readOnlyHint"], true);
        assert_eq!(list[1]["annotations"]["readOnlyHint"], false);
    }

    #[test]
    fn a_tool_call_is_handed_on_with_its_arguments() {
        let a = run(r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"add_slide","arguments":{"name":"Intro"}}}"#);
        assert_eq!(a, Action::Call { id: json!(3), name: "add_slide".into(), arguments: json!({ "name": "Intro" }) });
        let none = run(r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_project"}}"#);
        assert_eq!(none, Action::Call { id: json!(4), name: "get_project".into(), arguments: json!({}) }, "no arguments is an empty object");
    }

    #[test]
    fn a_call_to_a_tool_that_is_not_there_or_with_odd_arguments_is_refused() {
        assert_eq!(responded(run(r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"format_disk"}}"#))["error"]["code"], -32602);
        assert_eq!(responded(run(r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{}}"#))["error"]["code"], -32602);
        assert_eq!(responded(run(r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"add_slide","arguments":[1]}}"#))["error"]["code"], -32602);
    }

    #[test]
    fn ping_works_and_other_methods_are_not_offered() {
        assert_eq!(responded(run(r#"{"jsonrpc":"2.0","id":8,"method":"ping"}"#))["result"], json!({}));
        assert_eq!(responded(run(r#"{"jsonrpc":"2.0","id":9,"method":"resources/list"}"#))["error"]["code"], -32601);
    }

    #[test]
    fn broken_messages_are_answered_with_a_protocol_error() {
        assert_eq!(responded(run("{nope"))["error"]["code"], -32700);
        assert_eq!(responded(run(r#"[{"jsonrpc":"2.0","id":1,"method":"ping"}]"#))["error"]["code"], -32600, "a list of messages is not accepted");
        assert_eq!(responded(run(r#"{"jsonrpc":"2.0","id":1}"#))["error"]["code"], -32600);
        assert_eq!(responded(run("3"))["error"]["code"], -32600);
    }

    #[test]
    fn a_tool_that_fails_is_a_result_the_ai_tool_can_read() {
        let v = tool_result(json!(1), Err("There is no slide 9.".into()));
        assert_eq!(v["result"]["isError"], true);
        assert_eq!(v["result"]["content"][0], json!({ "type": "text", "text": "There is no slide 9." }));
        let ok = tool_result(json!(1), Ok("done".into()));
        assert_eq!(ok["result"]["isError"], false);
    }
}
