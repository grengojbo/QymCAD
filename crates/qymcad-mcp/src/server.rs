//! THE METHODS OF THE PROTOCOL: the handshake, the liveness probe, the lists a client reads to learn what it may
//! call, read and offer, the call of a tool, the reading of a resource and the getting of a prompt.

use std::io::{BufRead, Write};

use serde_json::{json, Value};

use crate::engine::{Engine, Looked, Window};
use crate::rpc::{self, Fault, Incoming};
use qymcad_tools::reading::{self, Unread};
use qymcad_tools::tool::{self, Ctx, Refusal};

/// The revisions of the protocol this server speaks, newest first. A client asking for one of them gets it
/// back; a client asking for anything else gets the newest, and decides itself whether to go on.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];

/// Answers every line of `input` on `output` until the input ends.
///
/// The output carries the protocol and nothing else: a stray line there is a message the client cannot read,
/// so what the server has to say about itself goes to stderr.
pub fn serve(input: impl BufRead, output: impl Write) -> std::io::Result<()> {
    serve_engine(Engine::Here(Box::new(Ctx::blank())), input, output)
}

/// Answers every line of `input` on `output` until the input ends, the calls going where `engine` sends them.
pub fn serve_engine(mut engine: Engine, mut input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
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
        if let Some(reply) = answer_engine(&mut engine, &text) {
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
    answer_to(Target::Here(ctx), line)
}

/// The reply to one line, the calls going to the window.
pub fn answer_window(window: &mut Window, line: &str) -> Option<Value> {
    answer_to(Target::Window(window), line)
}

/// The reply to one line, the calls going where `engine` sends them; an engine not decided yet looks for the window
/// before a call that reaches a document, and settles on its own document once a call has changed it.
pub fn answer_engine(engine: &mut Engine, line: &str) -> Option<Value> {
    let looked = if reaches_a_document(line) { engine.decide() } else { Looked::Decided };
    let reply = match (&mut *engine, looked) {
        (Engine::Undecided { .. }, Looked::Refused(refusal)) => answer_to(Target::Refused(refusal), line),
        (Engine::Here(ctx), _) => answer(ctx, line),
        (Engine::Window(window), _) => answer_window(window, line),
        (Engine::Undecided { own, .. }, _) => answer(own, line),
    };
    engine.settle();
    reply
}

/// Whether `line` asks for a document: a call of a tool or the reading of an address. The handshake and the lists do
/// not, and answering them decides nothing.
fn reaches_a_document(line: &str) -> bool {
    let method = serde_json::from_str::<Value>(line).ok().and_then(|v| v.get("method").and_then(Value::as_str).map(str::to_string));
    matches!(method.as_deref(), Some("tools/call" | "resources/read"))
}

/// Where one request is answered.
enum Target<'a> {
    Here(&'a mut Ctx),
    Window(&'a mut Window),
    /// A window listens and could not be reached: every request that reaches a document is refused with this.
    Refused(Refusal),
}

fn answer_to(target: Target<'_>, line: &str) -> Option<Value> {
    qymcad_i18n::set_language("en");
    match rpc::parse(line) {
        Incoming::Request { id, method, params } => Some(request(target, &id, &method, params)),
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

fn request(target: Target<'_>, id: &Value, method: &str, params: Value) -> Value {
    match method {
        "initialize" => rpc::success(id, initialize(&params)),
        "ping" => rpc::success(id, json!({})),
        "tools/list" => rpc::success(id, json!({ "tools": tool::listing() })),
        "tools/call" => call(target, id, params),
        "resources/list" => rpc::success(id, json!({ "resources": crate::resources::listing() })),
        "resources/templates/list" => rpc::success(id, json!({ "resourceTemplates": crate::resources::templates() })),
        "resources/read" => {
            let uri = params.get("uri").and_then(Value::as_str).unwrap_or_default();
            let read = match target {
                Target::Here(ctx) => reading::read(ctx, uri),
                Target::Window(window) => match window.pass(qymcad_tools::channel::READ, json!({ "uri": uri })) {
                    Ok(answer) => reading::answer_as_read(answer),
                    Err(cut) => Err(Unread::Nothing(format!("{} {}", cut.refusal.message, cut.refusal.hint.unwrap_or_default()))),
                },
                Target::Refused(refusal) => Err(Unread::Nothing(format!("{} {}", refusal.message, refusal.hint.unwrap_or_default()))),
            };
            match read {
                Ok(contents) => rpc::success(id, contents),
                Err(Unread::BadAddress(m)) => rpc::fault(id, Fault::InvalidParams, &m),
                Err(Unread::Nothing(m)) => rpc::fault(id, Fault::ResourceNotFound, &m),
            }
        }
        "prompts/list" => rpc::success(id, json!({ "prompts": crate::prompts::listing() })),
        "prompts/get" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            let given = params.get("arguments").and_then(Value::as_object).cloned().unwrap_or_default();
            match crate::prompts::get(name, &given) {
                Ok(prompt) => rpc::success(id, prompt),
                Err(why) => rpc::fault(id, Fault::InvalidParams, &why),
            }
        }
        _ => rpc::fault(id, Fault::MethodNotFound, &format!("no method {method}")),
    }
}

/// A tool by its name. A name the server does not have is a fault of the request, not a refusal of a tool.
/// The name is checked here, for the window too: the list of tools is the same on both ends.
fn call(target: Target<'_>, id: &Value, mut params: Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let Some(found) = tool::find(&name) else {
        return rpc::fault(id, Fault::InvalidParams, &format!("no tool {name}"));
    };
    let arguments = params.get_mut("arguments").map(Value::take).unwrap_or(Value::Null);
    match target {
        Target::Here(ctx) => rpc::success(id, tool::call(ctx, found, arguments)),
        Target::Window(window) => match window.pass(&name, arguments) {
            Ok(answer) => rpc::success(id, answer),
            Err(cut) => rpc::success(id, tool::refused_reply(&name, &cut.refusal, cut.after)),
        },
        Target::Refused(refusal) => rpc::success(id, tool::refused_reply(&name, &refusal, qymcad_tools::tool::After::Untouched)),
    }
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
