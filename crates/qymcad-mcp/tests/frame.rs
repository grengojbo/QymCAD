//! THE FRAME OVER EVERY TOOL: each one is listed with a schema a client can read, refuses arguments it does not
//! know before touching the document, and has a check of its own.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::{Ctx, ALL};
use serde_json::{json, Value};

fn request(ctx: &mut Ctx, method: &str, params: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }).to_string();
    answer(ctx, &line).expect("a request is answered")
}

/// EVERY TOOL IS LISTED as a client reads it: a name of lower-case words, words for the model, and a schema of an
/// object with its properties.
#[test]
fn every_tool_is_listed_with_a_schema() {
    let mut ctx = Ctx::blank();
    let listed = request(&mut ctx, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().expect("the tools are a list");
    assert_eq!(tools.len(), ALL.len(), "the list does not hold every tool");
    for t in tools {
        let name = t["name"].as_str().expect("a tool has a name");
        assert!(!name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "the name {name} is not lower-case words");
        assert!(t["description"].as_str().is_some_and(|d| d.len() > 20), "{name} says nothing about itself");
        let schema = &t["inputSchema"];
        assert_eq!(schema["type"], "object", "the schema of {name} is not an object: {schema}");
        assert!(schema["properties"].is_object(), "the schema of {name} has no properties: {schema}");
        assert_eq!(schema["additionalProperties"], json!(false), "the schema of {name} lets through fields the tool refuses: {schema}");
    }
}

/// AN ARGUMENT A TOOL DOES NOT KNOW is refused at the stage of validation, and the document stays as it was.
#[test]
fn an_unknown_argument_is_refused_before_the_work() {
    let mut ctx = Ctx::blank();
    for t in ALL {
        let reply = request(&mut ctx, "tools/call", json!({ "name": t.name, "arguments": { "no_such_field": 1 } }));
        let result = &reply["result"];
        assert_eq!(result["isError"], json!(true), "{} took an argument it does not know: {reply}", t.name);
        let body = &result["structuredContent"];
        assert_eq!(body["ok"], json!(false));
        assert_eq!(body["op"], t.name);
        assert_eq!(body["error"]["stage"], "validate", "{} refused it somewhere else: {body}", t.name);
        assert_eq!(body["rolled_back"], json!(true));
        let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().expect("the answer is also text")).expect("the text is the same JSON");
        assert_eq!(&text, body, "the text and the structured answer of {} differ", t.name);
    }
    assert!(ctx.doc.history().undo_names().is_empty(), "a refused call left a step");
}

/// A TOOL THE SERVER DOES NOT HAVE is a fault of the request, with the code JSON-RPC keeps for bad parameters.
#[test]
fn a_tool_that_is_not_there_is_a_fault_of_the_request() {
    let mut ctx = Ctx::blank();
    let reply = request(&mut ctx, "tools/call", json!({ "name": "no_such_tool", "arguments": {} }));
    assert_eq!(reply["error"]["code"], -32602, "{reply}");
}

/// EVERY TOOL HAS A CHECK OF ITS OWN: its name stands quoted in a check file other than this one. A tool written
/// and listed with no check is the one that breaks first and is noticed last.
#[test]
fn every_tool_has_a_check_of_its_own() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut checks = String::new();
    for e in std::fs::read_dir(&dir).expect("the checks read").flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "frame.rs") {
            checks += &std::fs::read_to_string(&p).expect("a check reads");
        }
    }
    let bare: Vec<&str> = ALL.iter().map(|t| t.name).filter(|n| !checks.contains(&format!("\"{n}\""))).collect();
    assert!(bare.is_empty(), "tools no check calls: {bare:?}");
}
