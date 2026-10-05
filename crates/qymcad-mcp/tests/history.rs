//! UNDO AND REDO through the server: a step is taken back and put again by name, and a history with nothing to take
//! answers with a refusal the model can read, not with a silent success.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": {} } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

#[test]
fn a_step_is_taken_back_and_put_again() {
    let mut ctx = Ctx::blank();
    let bodies = ctx.doc.project().bodies.len();
    let _ = ctx.doc.edit("cmd-box", |p| Ok(p.add_box(10.0, 20.0, 30.0))).expect("the box is laid");
    assert_eq!(ctx.doc.project().bodies.len(), bodies + 1);

    let undone = call(&mut ctx, "undo");
    assert_eq!(undone["ok"], json!(true), "{undone}");
    assert_eq!(undone["undone"]["step"], "cmd-box", "{undone}");
    assert!(undone["undone"]["words"].as_str().is_some_and(|w| !w.is_empty() && w != "cmd-box"), "the step has no words, only its code: {undone}");
    assert_eq!(ctx.doc.project().bodies.len(), bodies, "undo left the box");

    let redone = call(&mut ctx, "redo");
    assert_eq!(redone["redone"]["step"], "cmd-box", "{redone}");
    assert_eq!(ctx.doc.project().bodies.len(), bodies + 1, "redo did not put the box back");
}

/// A refusal of `tool` on an empty history, under `nothing-to-<tool>`.
fn refuses(tool: &str) {
    let mut ctx = Ctx::blank();
    let r = call(&mut ctx, tool);
    assert_eq!(r["ok"], json!(false), "{tool} on an empty history went through: {r}");
    assert_eq!(r["error"]["code"], format!("nothing-to-{tool}"));
    assert_eq!(r["error"]["stage"], "history");
}

#[test]
fn nothing_to_take_is_a_refusal() {
    refuses("undo");
    refuses("redo");
}
