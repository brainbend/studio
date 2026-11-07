//! The local server that AI tools connect to.
//!
//! It listens on this computer only (127.0.0.1) and answers MCP's "streamable HTTP" in its simplest form: a POST to
//! `/mcp` carries one JSON-RPC message and gets one JSON answer. Every request must carry the secret token. Browsers are
//! kept out as well, since a web page can reach a local server: a request whose `Origin` or `Host` is not this computer
//! is refused (that is what stops a page on the internet from using the controller through a rebinding trick).
//!
//! The server never touches the app's data. A tool call is sent to the app over a channel and the app answers it on its
//! own thread, one at a time.
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::protocol::{self, Action, ToolDef};

/// The most a message may weigh. Messages are small; a bigger one is a mistake or an attack.
const MAX_BODY: usize = 1024 * 1024;
/// How long to wait for the app to run a tool before telling the AI tool it is busy.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// A tool call for the app to run.
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
    /// Where the app sends the text of the answer, or the text of the error.
    pub reply: Sender<Result<String, String>>,
}

pub struct Config {
    /// The secret every request must carry as `Authorization: Bearer <token>`.
    pub token: String,
    /// The port to listen on; 0 lets the system choose.
    pub port: u16,
    pub version: String,
    /// What the AI tool is told about the controller when it connects.
    pub instructions: String,
}

/// A running server. It stops when this is dropped.
pub struct Handle {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Handle {
    pub fn port(&self) -> u16 { self.port }
    pub fn url(&self) -> String { format!("http://127.0.0.1:{}/mcp", self.port) }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() { let _ = t.join(); }
    }
}

/// A fresh secret: 32 random bytes as 64 hexadecimal digits.
pub fn new_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| format!("could not make a secret token: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Compares two secrets in the same time however much of them matches, so the time taken tells nothing about the token.
fn same_secret(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = (a.len() ^ b.len()) as u8;
    for i in 0..a.len().max(b.len()) { diff |= a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0); }
    diff == 0
}

/// Whether `host` (a `Host` header, with its port) is this computer on `port`.
fn local_host(host: &str, port: u16) -> bool {
    let host = host.trim().to_ascii_lowercase();
    host == format!("127.0.0.1:{port}") || host == format!("localhost:{port}")
}

/// Whether `origin` (an `Origin` header) is a page served from this computer.
fn local_origin(origin: &str) -> bool {
    let o = origin.trim().to_ascii_lowercase();
    let Some(rest) = o.strip_prefix("http://").or_else(|| o.strip_prefix("https://")) else { return false };
    let host = rest.split('/').next().unwrap_or("");
    let name = if let Some(v6) = host.strip_prefix('[') { v6.split(']').next().map(|h| format!("[{h}]")).unwrap_or_default() } else { host.split(':').next().unwrap_or("").to_string() };
    matches!(name.as_str(), "localhost" | "127.0.0.1" | "[::1]")
}

/// Why a request is refused: the HTTP status and what to say. `None` lets it through.
pub fn refuse(host: Option<&str>, origin: Option<&str>, authorization: Option<&str>, port: u16, token: &str) -> Option<(u16, &'static str)> {
    if !host.is_some_and(|h| local_host(h, port)) { return Some((403, "this server answers only requests addressed to this computer")); }
    if origin.is_some_and(|o| !local_origin(o)) { return Some((403, "this server does not answer web pages from other sites")); }
    let given = authorization.and_then(|a| a.strip_prefix("Bearer ")).unwrap_or("");
    if given.is_empty() || !same_secret(given.trim(), token) { return Some((401, "the token is missing or wrong; copy it again from BrainBend Studio's AI tools window")); }
    None
}

fn header(req: &tiny_http::Request, name: &str) -> Option<String> {
    req.headers().iter().find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name)).map(|h| h.value.as_str().to_string())
}

fn respond(req: tiny_http::Request, status: u16, body: Option<String>) {
    let mut r = tiny_http::Response::from_string(body.clone().unwrap_or_default()).with_status_code(status);
    if body.is_some() { r.add_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("a valid header")); }
    if status == 401 { r.add_header(tiny_http::Header::from_bytes(&b"WWW-Authenticate"[..], &b"Bearer"[..]).expect("a valid header")); }
    if status == 405 { r.add_header(tiny_http::Header::from_bytes(&b"Allow"[..], &b"POST"[..]).expect("a valid header")); }
    let _ = req.respond(r);
}

fn plain_error(req: tiny_http::Request, status: u16, message: &str) {
    respond(req, status, Some(protocol::error(Value::Null, -32000, message).to_string()));
}

/// Answers one request.
fn serve(mut req: tiny_http::Request, cfg: &Config, port: u16, tools: &[ToolDef], calls: &Sender<ToolCall>, wake: &(dyn Fn() + Send + Sync)) {
    let path = req.url().split('?').next().unwrap_or("").to_string();
    if let Some((status, message)) = refuse(header(&req, "Host").as_deref(), header(&req, "Origin").as_deref(), header(&req, "Authorization").as_deref(), port, &cfg.token) {
        return plain_error(req, status, message);
    }
    if path != "/mcp" { return plain_error(req, 404, "the controller is at /mcp"); }
    if *req.method() != tiny_http::Method::Post { return plain_error(req, 405, "send messages with POST"); }
    if req.body_length().is_some_and(|n| n > MAX_BODY) { return plain_error(req, 413, "the message is too big"); }
    let mut body = Vec::new();
    if req.as_reader().take(MAX_BODY as u64 + 1).read_to_end(&mut body).is_err() { return plain_error(req, 400, "could not read the message"); }
    if body.len() > MAX_BODY { return plain_error(req, 413, "the message is too big"); }
    match protocol::handle(&body, tools, &cfg.version, &cfg.instructions) {
        Action::Respond(v) => respond(req, 200, Some(v.to_string())),
        Action::Accepted => respond(req, 202, None),
        Action::Call { id, name, arguments } => {
            let (tx, rx) = channel();
            if calls.send(ToolCall { name, arguments, reply: tx }).is_err() { return respond(req, 200, Some(protocol::error(id, -32000, "BrainBend Studio is closing").to_string())); }
            wake();
            let answer = match rx.recv_timeout(CALL_TIMEOUT) {
                Ok(result) => protocol::tool_result(id, result),
                Err(_) => protocol::error(id, -32000, "BrainBend Studio did not answer in time; it may be waiting for you to close a window"),
            };
            respond(req, 200, Some(answer.to_string()));
        }
    }
}

/// Starts listening. Tool calls go to `calls`; `wake` is called after each so the app notices at once.
pub fn start(cfg: Config, tools: Vec<ToolDef>, calls: Sender<ToolCall>, wake: Arc<dyn Fn() + Send + Sync>) -> Result<Handle, String> {
    let server = tiny_http::Server::http(("127.0.0.1", cfg.port)).map_err(|e| format!("could not listen on port {}: {e}", cfg.port))?;
    let port = server.server_addr().to_ip().map(|a| a.port()).ok_or("the server has no port")?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let thread = std::thread::spawn(move || {
        while !flag.load(Ordering::Relaxed) {
            match server.recv_timeout(Duration::from_millis(100)) {
                Ok(Some(req)) => serve(req, &cfg, port, &tools, &calls, wake.as_ref()),
                Ok(None) => {}
                Err(_) => break,
            }
        }
    });
    Ok(Handle { port, stop, thread: Some(thread) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TOKEN: &str = "s3cret";

    #[test]
    fn only_requests_with_the_right_token_for_this_computer_get_through() {
        let ok = |host, origin, auth| refuse(host, origin, auth, 4000, TOKEN);
        assert_eq!(ok(Some("127.0.0.1:4000"), None, Some("Bearer s3cret")), None);
        assert_eq!(ok(Some("localhost:4000"), None, Some("Bearer s3cret")), None);
        assert_eq!(ok(Some("LOCALHOST:4000"), Some("http://localhost:5173"), Some("Bearer s3cret")), None, "a page on this computer is fine");
        assert_eq!(ok(Some("127.0.0.1:4000"), Some("http://[::1]:3000"), Some("Bearer s3cret")), None);
        assert_eq!(ok(None, None, Some("Bearer s3cret")).map(|r| r.0), Some(403), "no Host");
        assert_eq!(ok(Some("evil.example:4000"), None, Some("Bearer s3cret")).map(|r| r.0), Some(403), "a rebinding page's Host");
        assert_eq!(ok(Some("127.0.0.1:9999"), None, Some("Bearer s3cret")).map(|r| r.0), Some(403), "another port");
        assert_eq!(ok(Some("127.0.0.1:4000"), Some("https://evil.example"), Some("Bearer s3cret")).map(|r| r.0), Some(403), "a page from another site");
        assert_eq!(ok(Some("127.0.0.1:4000"), Some("http://localhost.evil.example"), Some("Bearer s3cret")).map(|r| r.0), Some(403), "a look-alike name");
        assert_eq!(ok(Some("127.0.0.1:4000"), Some("null"), Some("Bearer s3cret")).map(|r| r.0), Some(403));
        assert_eq!(ok(Some("127.0.0.1:4000"), None, None).map(|r| r.0), Some(401));
        assert_eq!(ok(Some("127.0.0.1:4000"), None, Some("Bearer wrong")).map(|r| r.0), Some(401));
        assert_eq!(ok(Some("127.0.0.1:4000"), None, Some("Bearer ")).map(|r| r.0), Some(401));
        assert_eq!(ok(Some("127.0.0.1:4000"), None, Some("s3cret")).map(|r| r.0), Some(401), "the word Bearer is needed");
        assert_eq!(ok(Some("127.0.0.1:4000"), None, Some("Bearer s3cre")).map(|r| r.0), Some(401), "a prefix is not the token");
    }

    #[test]
    fn secrets_are_compared_whole() {
        assert!(same_secret("abc", "abc"));
        assert!(!same_secret("abc", "abd") && !same_secret("abc", "ab") && !same_secret("", "a") && !same_secret("abc", "abcd"));
    }

    #[test]
    fn tokens_are_long_hex_and_never_repeat() {
        let (a, b) = (new_token().unwrap(), new_token().unwrap());
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    // ---- over a real connection ----

    fn tools() -> Vec<ToolDef> {
        vec![
            ToolDef { name: "echo", description: "Says back its text", schema: json!({ "type": "object", "properties": { "text": { "type": "string" } } }), read_only: true },
            ToolDef { name: "fail", description: "Always fails", schema: json!({ "type": "object" }), read_only: true },
        ]
    }

    /// A server, with a stand-in for the app that answers tool calls.
    fn running(port: u16) -> (Handle, std::thread::JoinHandle<()>, String) {
        let (tx, rx) = channel::<ToolCall>();
        let cfg = Config { token: TOKEN.into(), port, version: "0.3.0".into(), instructions: "Use with care.".into() };
        let handle = start(cfg, tools(), tx, Arc::new(|| {})).expect("the server starts");
        let app = std::thread::spawn(move || {
            while let Ok(call) = rx.recv() {
                let answer = match call.name.as_str() {
                    "echo" => Ok(call.arguments["text"].as_str().unwrap_or("").to_string()),
                    _ => Err("it failed on purpose".to_string()),
                };
                let _ = call.reply.send(answer);
            }
        });
        let url = handle.url();
        (handle, app, url)
    }

    fn post(url: &str, auth: Option<&str>, body: &str) -> (u16, String) {
        let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
        let mut req = agent.post(url).header("Content-Type", "application/json");
        if let Some(a) = auth { req = req.header("Authorization", a); }
        let mut reply = req.send(body.as_bytes()).expect("the server answers");
        (reply.status().as_u16(), reply.body_mut().read_to_string().unwrap_or_default())
    }

    #[test]
    fn a_connection_can_start_list_tools_and_call_them() {
        let (handle, app, url) = running(0);
        let auth = Some("Bearer s3cret");
        let (status, body) = post(&url, auth, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#);
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!((status, v["result"]["serverInfo"]["name"].as_str(), v["result"]["instructions"].as_str()), (200, Some("brainbend-studio"), Some("Use with care.")));
        let (status, body) = post(&url, auth, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        assert_eq!((status, body.as_str()), (202, ""));
        let (_, body) = post(&url, auth, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["result"]["tools"].as_array().unwrap().len(), 2);
        let (_, body) = post(&url, auth, r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"text":"hello"}}}"#);
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!((v["id"].as_i64(), v["result"]["isError"].as_bool(), v["result"]["content"][0]["text"].as_str()), (Some(3), Some(false), Some("hello")));
        let (_, body) = post(&url, auth, r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"fail"}}"#);
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!((v["result"]["isError"].as_bool(), v["result"]["content"][0]["text"].as_str()), (Some(true), Some("it failed on purpose")));
        drop(handle);
        let _ = app.join();
    }

    #[test]
    fn requests_that_are_not_allowed_never_reach_the_app() {
        let (handle, app, url) = running(0);
        let call = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{"text":"x"}}}"#;
        assert_eq!(post(&url, None, call).0, 401, "no token");
        assert_eq!(post(&url, Some("Bearer nope"), call).0, 401, "wrong token");
        let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
        let r = agent.post(&url).header("Authorization", "Bearer s3cret").header("Origin", "https://evil.example").send(call.as_bytes()).unwrap();
        assert_eq!(r.status().as_u16(), 403, "a web page from another site");
        let r = agent.get(&url).header("Authorization", "Bearer s3cret").call().unwrap();
        assert_eq!(r.status().as_u16(), 405, "GET is not how messages are sent");
        let r = agent.post(&url.replace("/mcp", "/other")).header("Authorization", "Bearer s3cret").send(call.as_bytes()).unwrap();
        assert_eq!(r.status().as_u16(), 404);
        let big = "x".repeat(MAX_BODY + 10);
        assert_eq!(post(&url, Some("Bearer s3cret"), &big).0, 413, "too big");
        let (status, body) = post(&url, Some("Bearer s3cret"), "{nope");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!((status, v["error"]["code"].as_i64()), (200, Some(-32700)), "bad JSON is a protocol error");
        drop(handle);
        let _ = app.join();
    }

    #[test]
    fn stopping_the_server_frees_its_port() {
        let (handle, app, _) = running(0);
        let port = handle.port();
        assert!(port > 0);
        drop(handle);
        let _ = app.join();
        let (again, app2, _) = running(port);
        assert_eq!(again.port(), port, "the port can be used again");
        drop(again);
        let _ = app2.join();
    }
}
