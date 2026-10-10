//! THE FRAME EVERY TOOL GOES THROUGH. A tool reads its arguments into a typed record - the reading IS the
//! validation - does its work on the one document, and answers with a JSON object. The frame adds what every answer
//! carries (`ok`, `op`), turns a refusal into the protocol's tool error, and outlives a panic: the action the panic
//! broke is taken back and the server goes on.
//!
//! A refusal is an answer, not a fault of the protocol: the model reads `ok: false` with its code and tries again,
//! where a JSON-RPC error would read as a broken server.

use qymcad_doc::{DocEngine, DocError};
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};

/// What a tool works on: the one document the server keeps, the file it was opened from or last saved to, what a
/// person has selected in a window, when there is one, and the language of the person's interface.
pub struct Ctx {
    pub doc: DocEngine,
    pub path: Option<String>,
    pub seen: Seen,
    /// The code of the language the person's QymCAD interface is in (`en`, `uk`, `ru`).
    pub language: String,
}

/// WHAT A PERSON HAS SELECTED, as far as the tools can know.
#[derive(Clone)]
pub enum Seen {
    /// A document of the program's own: nobody is looking at it.
    NoWindow,
    /// The open window's document: what is selected in it - possibly nothing - and the eye it is seen through.
    Window(InWindow),
}

/// WHAT THE OPEN WINDOW TELLS THE TOOLS: what is selected, and the eye the person looks through.
#[derive(Clone)]
pub struct InWindow {
    pub picked: Vec<crate::tools::selection::Picked>,
    pub eye: crate::picture::Eye,
}

impl Ctx {
    /// A new document with one part, as the window starts.
    pub fn blank() -> Self {
        Ctx { doc: DocEngine::blank(), path: None, seen: Seen::NoWindow, language: crate::person::language() }
    }
}

/// What a tool answers with when it went through: the fields of its own answer, beside `ok` and `op`.
pub type Answer = Map<String, Value>;

/// THE KEY OF A PICTURE in an answer: a PNG in base64. It leaves the JSON and goes to the client as an image of its
/// own - a picture as text would be thousands of characters the model reads as nothing.
pub const PICTURE: &str = "png";

/// The argument any tool but `render` takes besides its own: draw the document after the change, from the default
/// view, and hand the picture back with the answer.
const RENDER_TOO: &str = "render";

/// ONE TOOL: its name and words for the model, the schema of its arguments, and the work.
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: fn() -> Value,
    pub call: fn(&mut Ctx, Value) -> Result<Answer, Refusal>,
}

/// Where a call broke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    /// The arguments do not fit the schema.
    Validate,
    /// The history has no step to take.
    History,
    /// A file could not be read or written, or is not the kind the tool takes.
    Io,
    /// The action laid a node on what the document no longer holds.
    Resolve,
    /// The work itself broke: the kernel refused or the program panicked.
    Kernel,
    /// The call did not reach the window's document, or its answer did not come back.
    Window,
}

impl Stage {
    pub fn word(self) -> &'static str {
        match self {
            Stage::Validate => "validate",
            Stage::History => "history",
            Stage::Io => "io",
            Stage::Resolve => "resolve",
            Stage::Kernel => "kernel",
            Stage::Window => "window",
        }
    }
}

/// WHY A TOOL DID NOT GO THROUGH: a stable code to branch on, a sentence to read, where it broke, and what to try.
#[derive(Debug)]
pub struct Refusal {
    pub code: String,
    pub message: String,
    pub stage: Stage,
    pub hint: Option<String>,
}

impl Refusal {
    pub fn new(code: &str, message: &str, stage: Stage) -> Self {
        Refusal { code: code.to_string(), message: message.to_string(), stage, hint: None }
    }

    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_string());
        self
    }
}

/// A REFUSAL OF THE DOCUMENT, in the words of the catalogue: its code is the head of what the document said, up to
/// the argument after `#`, and its message is that in English.
pub fn doc_refusal(e: DocError) -> Refusal {
    let coded = |stored: &str, stage: Stage| {
        let head = stored.split('#').next().unwrap_or(stored);
        let code = if head.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') { head } else { "refused" };
        Refusal::new(code, &qymcad_i18n::name(stored), stage)
    };
    match e {
        DocError::File(stored) => coded(&stored, Stage::Io),
        DocError::Refused(stored) => coded(&stored, Stage::Kernel),
        DocError::Gone(nodes) => {
            Refusal::new("inputs-gone", &format!("The action would stand on what the document no longer holds: nodes {nodes:?}."), Stage::Resolve).with_hint("Read get_document for what is there now.")
        }
    }
}

/// READ THE ARGUMENTS into the tool's record. A field the record does not know, a field missing, a value of the wrong
/// kind - each is refused here, before the document is touched. No arguments at all read as an empty object.
pub fn args<T: DeserializeOwned>(arguments: Value) -> Result<T, Refusal> {
    let arguments = if arguments.is_null() { json!({}) } else { arguments };
    serde_json::from_value(arguments).map_err(|e| Refusal::new("arguments", &e.to_string(), Stage::Validate).with_hint("Compare the arguments with the tool's inputSchema."))
}

/// THE TOOLS, in the order `tools/list` gives them.
pub const ALL: &[Tool] = &[
    crate::tools::doc::NEW_PROJECT,
    crate::tools::doc::OPEN_PROJECT,
    crate::tools::doc::SAVE_PROJECT,
    crate::tools::doc::GET_DOCUMENT,
    crate::tools::params::SET_PARAMETER,
    crate::tools::params::DELETE_PARAMETER,
    crate::tools::params::LIST_PARAMETERS,
    crate::tools::exchange::IMPORT_MESH,
    crate::tools::exchange::IMPORT_CAD,
    crate::tools::exchange::EXPORT_MESH,
    crate::tools::exchange::EXPORT_CAD,
    crate::tools::prim::BOX,
    crate::tools::prim::CYLINDER,
    crate::tools::prim::SPHERE,
    crate::tools::prim::CONE,
    crate::tools::prim::TORUS,
    crate::tools::prim::PRISM,
    crate::tools::sketch::CREATE_SKETCH,
    crate::tools::sketch::SKETCH_ADD,
    crate::tools::sketch::SKETCH_INFO,
    crate::tools::features::EXTRUDE,
    crate::tools::features::REVOLVE,
    crate::tools::features::FILLET,
    crate::tools::features::CHAMFER,
    crate::tools::features::HOLE,
    crate::tools::features::SHELL,
    crate::tools::modify::LINEAR_PATTERN,
    crate::tools::modify::CIRCULAR_PATTERN,
    crate::tools::modify::MIRROR,
    crate::tools::modify::BOOLEAN,
    crate::tools::modify::MOVE,
    crate::tools::modify::SPLIT_BODY,
    crate::tools::modify::PUSH_FACE,
    crate::tools::timeline::ADD_PART,
    crate::tools::timeline::SET_ACTIVE_PART,
    crate::tools::timeline::DELETE_FEATURE,
    crate::tools::timeline::SUPPRESS_FEATURE,
    crate::tools::timeline::EDIT_FEATURE,
    crate::tools::history::UNDO,
    crate::tools::history::REDO,
    crate::tools::look::RENDER,
    crate::tools::query::LIST_FACES,
    crate::tools::query::LIST_EDGES,
    crate::tools::query::RESOLVE,
    crate::tools::query::INSPECT,
    crate::tools::query::MEASURE,
    crate::tools::selection::GET_SELECTION,
    crate::tools::batch::APPLY_OPS,
];

pub fn find(name: &str) -> Option<&'static Tool> {
    ALL.iter().find(|t| t.name == name)
}

/// The list a client reads to learn what it may call.
pub fn listing() -> Value {
    Value::Array(ALL.iter().map(|t| json!({ "name": t.name, "description": t.description, "inputSchema": schema(t) })).collect())
}

/// The schema of a tool's arguments, with `render` added to every tool but the one that draws.
fn schema(tool: &Tool) -> Value {
    let mut schema = (tool.schema)();
    if tool.name != crate::tools::look::RENDER.name {
        if let Some(props) = schema.get_mut("properties").and_then(Value::as_object_mut) {
            props.insert(
                RENDER_TOO.into(),
                json!({ "type": "boolean", "default": false, "description": "Also draw the document after the call (iso, 800 x 600) and give the picture with the answer." }),
            );
        }
    }
    schema
}

/// Whether the call asks for a picture too; the argument is taken out, so the tool reads only its own.
fn wants_picture(tool: &Tool, arguments: &mut Value) -> bool {
    if tool.name == crate::tools::look::RENDER.name {
        return false;
    }
    let taken = arguments.as_object_mut().and_then(|a| a.remove(RENDER_TOO));
    taken == Some(json!(true))
}

/// CALL `tool` and answer as the protocol answers a tool: the JSON as text for any client, the same as structured
/// content for a client that reads it, and `isError` on a refusal.
pub fn call(ctx: &mut Ctx, tool: &Tool, mut arguments: Value) -> Value {
    let picture_too = wants_picture(tool, &mut arguments);
    // the kernel's own words for what it refused are kept on this thread - the rebuild runs here - until read; a
    // refusal left from an earlier call must not be told as this one's
    qymcad_kernel::clear_kernel_refusal();
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (tool.call)(ctx, arguments)));
    let body = match ran {
        Ok(Ok(mut answer)) => {
            if picture_too {
                // a document with nothing to draw answers without a picture: the call itself went through
                let look = crate::picture::Look {
                    view: crate::tools::look::default_view(ctx),
                    size: crate::picture::Size { width: 800, height: 600 },
                    lit: crate::picture::Lit::Nothing,
                    edges: crate::picture::Edges::Drawn,
                    window: crate::tools::look::eye(ctx),
                };
                if let Ok(picture) = crate::tools::look::picture_answer(ctx, &look) {
                    answer.insert("picture".into(), Value::Object(picture));
                }
            }
            let mut body = Map::new();
            body.insert("ok".into(), json!(true));
            body.insert("op".into(), json!(tool.name));
            body.extend(answer);
            Value::Object(body)
        }
        // a refusal is made before the document changes, or by an action the document takes back whole
        Ok(Err(refusal)) => refused(tool, &refusal, true),
        Err(panic) => {
            let rolled_back = ctx.doc.recover();
            let what = panic.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| panic.downcast_ref::<String>().cloned()).unwrap_or_default();
            let refusal = Refusal::new("kernel-panic", &format!("The work broke inside the program: {what}"), Stage::Kernel)
                .with_hint("The document is as it was before the call. Try other values, or another way to the same shape.");
            refused(tool, &refusal, rolled_back)
        }
    };
    let error = body["ok"] == json!(false);
    let mut body = body;
    // THE KERNEL'S OWN WORDS, beside the coded reason, where the call went wrong: refused, or a feature left red. On a
    // call that went through they are left out - the kernel may refuse one way and succeed another.
    let went_wrong = error || body["red"].as_array().is_some_and(|r| !r.is_empty());
    if let (true, Some(said)) = (went_wrong, qymcad_kernel::last_kernel_refusal()) {
        body["kernel_said"] = json!(said);
    }
    let png = take_picture(&mut body);
    let mut content = vec![json!({ "type": "text", "text": body.to_string() })];
    if let Some(data) = png {
        content.push(json!({ "type": "image", "data": data, "mimeType": "image/png" }));
    }
    json!({ "content": content, "structuredContent": body, "isError": error })
}

/// The picture out of an answer, wherever the answer holds it: at its top (`render`) or in `picture` (`render: true`).
fn take_picture(body: &mut Value) -> Option<Value> {
    let obj = body.as_object_mut()?;
    obj.remove(PICTURE).or_else(|| obj.get_mut("picture").and_then(Value::as_object_mut).and_then(|p| p.remove(PICTURE)))
}

fn refused(tool: &Tool, refusal: &Refusal, rolled_back: bool) -> Value {
    let after = if rolled_back { After::Untouched } else { After::Left };
    refused_body(tool.name, refusal, after)
}

/// WHAT A REFUSED CALL LEFT OF THE DOCUMENT.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum After {
    /// The document is as it was before the call: the call never started, or was taken back whole.
    Untouched,
    /// The call broke outside an action: the document is as far as it got, and nothing was taken back.
    Left,
    /// Nobody can tell: the call may have gone through where its answer was lost.
    Unknown,
}

fn refused_body(name: &str, refusal: &Refusal, after: After) -> Value {
    let mut error = json!({ "code": refusal.code, "message": refusal.message, "stage": refusal.stage.word() });
    if let Some(hint) = &refusal.hint {
        error["hint"] = json!(hint);
    }
    let rolled_back = match after {
        After::Untouched => json!(true),
        After::Left => json!(false),
        After::Unknown => Value::Null,
    };
    json!({ "ok": false, "op": name, "error": error, "rolled_back": rolled_back })
}

/// A REFUSAL OF A CALL AS THE PROTOCOL ANSWERS A TOOL, for a call refused before any tool ran: by name, with the
/// document `after` it.
pub fn refused_reply(name: &str, refusal: &Refusal, after: After) -> Value {
    let body = refused_body(name, refusal, after);
    json!({ "content": [{ "type": "text", "text": body.to_string() }], "structuredContent": body, "isError": true })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaks(ctx: &mut Ctx, _: Value) -> Result<Answer, Refusal> {
        ctx.doc
            .edit("box", |p| -> Result<Answer, String> {
                let _ = p.add_box(5.0, 5.0, 5.0);
                panic!("a panic for the check")
            })
            .map_err(|_| Refusal::new("unreachable", "", Stage::Kernel))
    }

    /// A PANIC IN THE MIDDLE OF AN ACTION is answered as a refusal with its code, the action is taken back, and the
    /// document takes the next call as if nothing happened.
    #[test]
    fn a_panic_is_a_refusal_and_the_action_is_taken_back() {
        let tool = Tool { name: "breaks", description: "", schema: || json!({}), call: breaks };
        let mut ctx = Ctx::blank();
        let bodies = ctx.doc.project().bodies.len();
        let reply = call(&mut ctx, &tool, Value::Null);
        assert_eq!(reply["isError"], json!(true), "a panic went out as success: {reply}");
        let body = &reply["structuredContent"];
        assert_eq!(body["error"]["code"], "kernel-panic");
        assert_eq!(body["rolled_back"], json!(true), "the broken action was not taken back: {body}");
        assert!(body["error"]["message"].as_str().is_some_and(|m| m.contains("a panic for the check")), "the panic's words were lost: {body}");
        assert_eq!(ctx.doc.project().bodies.len(), bodies, "the broken action left a body behind");
        assert!(ctx.doc.history().undo_names().is_empty(), "the broken action left a step behind");
        assert!(!ctx.doc.history().is_open(), "the broken action is still open");
    }
}
