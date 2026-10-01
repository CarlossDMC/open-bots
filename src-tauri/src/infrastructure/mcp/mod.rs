//! Local MCP server that offers the runtime tools to provider CLIs during a turn.
//!
//! Transport: MCP Streamable HTTP, POST only, with JSON responses (no SSE stream). It binds to
//! `127.0.0.1` on a random port, requires a per-turn bearer token, and rejects requests with
//! a non-loopback `Origin` header. See ADR 0008.

mod protocol;
mod server;

pub use protocol::{handle_message, SUPPORTED_PROTOCOL_VERSIONS};
pub use server::{McpListener, MAX_REQUEST_BYTES};
