//! BrainBend Studio's MCP controller: the part that talks to AI tools.
//!
//! MCP (the Model Context Protocol) lets an AI tool of the author's choice use an app through a list of tools. This crate
//! is the protocol ([`protocol`]) and the local server ([`server`]); it knows nothing about slides. The app gives it the
//! tools it offers and answers each tool call on its own thread.
pub mod protocol;
pub mod server;

pub use protocol::{Action, ToolDef};
pub use server::{new_token, refuse, start, Config, Handle, ToolCall};
