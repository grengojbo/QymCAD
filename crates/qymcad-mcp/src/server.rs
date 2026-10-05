//! THE METHODS OF THE PROTOCOL: the handshake, the liveness probe, the three lists a client reads to learn what it
//! may call, and the call of a tool.

use std::io::{BufRead, Write};

use serde_json::{json, Value};

use crate::rpc::{self, Fault, Incoming};
use crate::tool::{self, Ctx};

/// The revisions of the protocol this server speaks, newest first. A client asking for one of them gets it
/// back; a client asking for anything else gets the newest, and decides itself whether to go on.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];

/// Answers every line of `input` on `output` until the input ends.
///
/// The output carries the protocol and nothing else: a stray line there is a message the client cannot read,
/// so what the server has to say about itself goes to stderr.
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    let mut ctx = Ctx::blank();
    let mut line = Vec::new();
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line)? == 0 {
            return Ok(());
        }
        // bytes that are not UTF-8 are still a line, and the parse answers it as one that is not JSON
        let text = String::from_utf8_lossy(&line);
        if text.trim().is_empty() {
            continue;
        }
        if let Some(reply) = answer(&mut ctx, &text) {
            serde_json::to_writer(&mut output, &reply)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}

/// The reply to one line, or nothing when the line is owed none.
///
/// EVERY REPLY IS ENGLISH. The language is bound to the thread, and left alone it follows the system locale: on a
/// machine set to Ukrainian a refusal came back in Ukrainian once Ukrainian joined the catalogue. The reader is a
/// model, and one language keeps its replies alike from one machine to the next, so it is set here, on whatever thread
/// asks, rather than once at the start of the program.
pub fn answer(ctx: &mut Ctx, line: &str) -> Option<Value> {
    qymcad_i18n::set_language("en");
    match rpc::parse(line) {
        Incoming::Request { id, method, params } => Some(request(ctx, &id, &method, params)),
        Incoming::Notification { method } => {
            // `notifications/initialized` and `notifications/cancelled` need no work here: requests are
            // answered one at a time, so one being cancelled has already been answered
            eprintln!("qymcad-mcp: notification {method}");
            None
        }
        Incoming::Reply => None,
        Incoming::Broken(reply) => {
            eprintln!("qymcad-mcp: a line that is no request: {}", line.trim());
            Some(reply)
        }
    }
}

fn request(ctx: &mut Ctx, id: &Value, method: &str, params: Value) -> Value {
    match method {
        "initialize" => rpc::success(id, initialize(&params)),
        "ping" => rpc::success(id, json!({})),
        "tools/list" => rpc::success(id, json!({ "tools": tool::listing() })),
        "tools/call" => call(ctx, id, params),
        "resources/list" => rpc::success(id, json!({ "resources": [] })),
        "prompts/list" => rpc::success(id, json!({ "prompts": [] })),
        _ => rpc::fault(id, Fault::MethodNotFound, &format!("no method {method}")),
    }
}

/// A tool by its name. A name the server does not have is a fault of the request, not a refusal of a tool.
fn call(ctx: &mut Ctx, id: &Value, mut params: Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let Some(found) = tool::find(&name) else {
        return rpc::fault(id, Fault::InvalidParams, &format!("no tool {name}"));
    };
    let arguments = params.get_mut("arguments").map(Value::take).unwrap_or(Value::Null);
    rpc::success(id, tool::call(ctx, found, arguments))
}

/// The handshake: the revision both sides speak, what the server offers and what it is called.
fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = PROTOCOL_VERSIONS.into_iter().find(|v| Some(*v) == asked).unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
        "serverInfo": { "name": "qymcad-mcp", "version": env!("CARGO_PKG_VERSION") },
    })
}
