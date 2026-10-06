//! A BATCH OF OPERATIONS through the server: one undo step for the whole; each operation reads what the one before it
//! built - a chamfer of the top of a block laid in the same batch takes the four edges of that top, not every edge;
//! a refusal anywhere takes the batch back whole and leaves no step; a later operation reads a named one's answer.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

fn ok(reply: Value) -> Value {
    assert_eq!(reply["ok"], json!(true), "{reply}");
    reply
}

/// The volumes of the bodies standing, in the order of the document.
fn volumes(ctx: &mut Ctx) -> Vec<f64> {
    let doc = ok(call(ctx, "get_document", json!({})));
    doc["document"]["bodies"].as_array().expect("bodies").iter().filter(|b| b["consumed"] == json!(false)).map(|b| b["volume"].as_f64().expect("a volume")).collect()
}

fn steps(ctx: &Ctx) -> Vec<String> {
    ctx.doc.history().undo_names().into_iter().map(str::to_string).collect()
}

/// A BLOCK AND A CHAMFER OF ITS TOP IN ONE BATCH take the four edges of that top: a chamfer of 1.5 along 140 mm of
/// rim takes 0.5 x 1.5^2 x 140 less the four corners where two legs overlap, 4 x 1.5^3 / 3 - 157.5 - 4.5 = 153
/// mm^3. Every edge of the block would be 342.
#[test]
fn a_chamfer_laid_with_its_block_takes_the_edges_of_its_top() {
    let mut ctx = Ctx::blank();
    let done = ok(call(
        &mut ctx,
        "apply_ops",
        json!({ "ops": [
            { "tool": "box", "args": { "x": 40, "y": 30, "z": 10 }, "as": "block" },
            { "tool": "chamfer", "args": { "body": { "body": "$block.body" }, "edges": { "adjacent": "top" }, "distance": 1.5 } },
        ] }),
    ));
    let taken = 12000.0 - volumes(&mut ctx)[0];
    assert!((taken - 153.0).abs() < 1e-3, "the chamfer took {taken} mm^3, not 153: {done}");
    assert_eq!(steps(&ctx), vec!["status-ops-batch".to_string()], "the batch is not one step");
    let _ = ok(call(&mut ctx, "undo", json!({})));
    assert!(volumes(&mut ctx).is_empty(), "one undo did not take the whole batch back");
}

/// A REFUSAL ON THE THIRD OPERATION takes the first two back with it: the document is as it was before the call,
/// and no step is left.
#[test]
fn a_refusal_takes_the_batch_back_whole() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "cylinder", json!({ "radius": 5, "height": 5 })));
    let before = volumes(&mut ctx);
    let steps_before = steps(&ctx);
    let refused = call(
        &mut ctx,
        "apply_ops",
        json!({ "ops": [
            { "tool": "add_part", "args": {} },
            { "tool": "box", "args": { "x": 40, "y": 30, "z": 10 } },
            { "tool": "fillet", "args": { "edges": { "adjacent": "top" }, "radius": -1 } },
        ] }),
    );
    assert_eq!(refused["ok"], json!(false), "{refused}");
    assert_eq!(refused["rolled_back"], json!(true), "{refused}");
    let said = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(said.starts_with("Operation 3 (fillet)"), "the refusal does not name the operation: {refused}");
    assert_eq!(volumes(&mut ctx), before, "the batch left something behind");
    let doc = ok(call(&mut ctx, "get_document", json!({})));
    assert_eq!(doc["document"]["parts"].as_array().map(Vec::len), Some(1), "the part the batch added stayed: {doc}");
    assert_eq!(steps(&ctx), steps_before, "a refused batch left a step");
}

/// A LATER OPERATION READS A NAMED ONE'S ANSWER: the sketch on the block's top, drawn and cut through, all by name.
#[test]
fn a_later_operation_reads_a_named_answer() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(
        &mut ctx,
        "apply_ops",
        json!({ "name": "pocket", "ops": [
            { "tool": "box", "args": { "x": 40, "y": 30, "z": 10 }, "as": "block" },
            { "tool": "create_sketch", "args": { "plane": { "body": { "body": "$block.body" }, "face": "top" } }, "as": "top" },
            { "tool": "sketch_add", "args": { "sketch": "$top.sketch", "entities": [{ "rect": { "from": [-5, -5], "to": [5, 5] } }] } },
            { "tool": "extrude", "args": { "sketch": "$top.sketch", "distance": 4, "op": "cut" } },
        ] }),
    ));
    assert!((volumes(&mut ctx)[0] - (12000.0 - 400.0)).abs() < 1e-6, "the pocket 10 x 10 x 4 was not cut");
    assert_eq!(steps(&ctx), vec!["pocket".to_string()], "the batch's own name is not the step's");

    let mut ctx = Ctx::blank();
    let unknown = call(
        &mut ctx,
        "apply_ops",
        json!({ "ops": [{ "tool": "box", "args": { "x": 1, "y": 1, "z": 1 } }, { "tool": "fillet", "args": { "body": { "body": "$nobody.body" }, "edges": "all", "radius": 0.1 } }] }),
    );
    assert_eq!(unknown["error"]["code"], "unknown-name", "{unknown}");
    assert!(volumes(&mut ctx).is_empty(), "the box of a refused batch stayed");
}

#[test]
fn what_cannot_be_taken_back_with_the_batch_is_refused() {
    let mut ctx = Ctx::blank();
    for tool in ["undo", "save_project", "apply_ops"] {
        let r = call(&mut ctx, "apply_ops", json!({ "ops": [{ "tool": "box", "args": { "x": 1, "y": 1, "z": 1 } }, { "tool": tool, "args": {} }] }));
        assert_eq!(r["error"]["code"], "not-in-batch", "{tool}: {r}");
        assert!(volumes(&mut ctx).is_empty(), "{tool}: the box stayed");
    }
    let r = call(&mut ctx, "apply_ops", json!({ "ops": [{ "tool": "no_such_tool" }] }));
    assert_eq!(r["error"]["code"], "no-tool", "{r}");
}
