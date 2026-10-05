//! JSON-RPC 2.0 AS THE MODEL CONTEXT PROTOCOL FRAMES IT ON STDIO: one message per line, UTF-8, no newline
//! inside a message. A request carries an `id` and is owed exactly one reply; a notification carries none and
//! is owed nothing, not even an error.
//!
//! Batches (a JSON array of messages) were dropped from the protocol in its 2025-06-18 revision, so an array is
//! answered as an invalid request rather than unpacked.

use serde_json::{json, Value};

/// The codes JSON-RPC 2.0 reserves for a message the server could not take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fault {
    /// The line is not JSON at all.
    Parse,
    /// JSON, but not a request: no `jsonrpc: "2.0"`, no `method`, an array.
    InvalidRequest,
    /// A request for a method this server does not have.
    MethodNotFound,
}

impl Fault {
    pub fn code(self) -> i64 {
        match self {
            Fault::Parse => -32700,
            Fault::InvalidRequest => -32600,
            Fault::MethodNotFound => -32601,
        }
    }
}

/// One message read off the wire.
pub enum Incoming {
    /// A request: it is owed one reply under the same `id`.
    Request { id: Value, method: String, params: Value },
    /// A notification: nothing goes back.
    Notification { method: String },
    /// A reply to a request of ours. The server sends none, so whatever comes is dropped.
    Reply,
    /// A line that is no message; the reply is ready to send, under `id: null` when the id could not be read.
    Broken(Value),
}

/// Reads one line of the wire into a message.
pub fn parse(line: &str) -> Incoming {
    let Ok(msg) = serde_json::from_str::<Value>(line) else {
        return Incoming::Broken(fault(&Value::Null, Fault::Parse, "the line is not JSON"));
    };
    let Some(obj) = msg.as_object() else {
        return Incoming::Broken(fault(&Value::Null, Fault::InvalidRequest, "a message is a JSON object; batches are not part of the protocol"));
    };
    let id = obj.get("id").cloned();
    if obj.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Incoming::Broken(fault(id.as_ref().unwrap_or(&Value::Null), Fault::InvalidRequest, "the message does not say jsonrpc: 2.0"));
    }
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        if obj.contains_key("result") || obj.contains_key("error") {
            return Incoming::Reply;
        }
        return Incoming::Broken(fault(id.as_ref().unwrap_or(&Value::Null), Fault::InvalidRequest, "the message names no method"));
    };
    let method = method.to_string();
    match id {
        // an id is a string or a number; null is allowed by JSON-RPC and forbidden by the protocol
        Some(id) if id.is_string() || id.is_number() => Incoming::Request { id, method, params: obj.get("params").cloned().unwrap_or(Value::Null) },
        Some(_) => Incoming::Broken(fault(&Value::Null, Fault::InvalidRequest, "a request id is a string or a number")),
        None => Incoming::Notification { method },
    }
}

/// The reply to a request that went through.
pub fn success(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// The reply to a request the server could not take.
pub fn fault(id: &Value, fault: Fault, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": fault.code(), "message": message } })
}
