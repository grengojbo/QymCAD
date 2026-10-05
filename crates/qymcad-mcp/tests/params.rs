//! THE PARAMETERS THROUGH THE SERVER: a block whose height reads `h` is rebuilt when `h` changes and comes back with
//! undo; a typo in an expression is refused before the document changes; a deleted parameter turns red what reads it.

use qymcad_core::feature::{Purpose, Reach};
use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

/// A 40 x 30 block whose height is the expression `h`, with `h = 10`.
fn block_of_height_h() -> Ctx {
    let mut ctx = Ctx::blank();
    let set = call(&mut ctx, "set_parameter", json!({ "name": "h", "expr": "10" }));
    assert_eq!(set["ok"], json!(true), "{set}");
    let _ = ctx
        .doc
        .edit("block", |p| {
            let si = p.new_sketch("base");
            let sid = p.sketches[si].id;
            p.add_sketch_node(sid, "base");
            let _ = p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, Purpose::Real);
            p.regen_sketch(si);
            let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("a closed contour");
            let e = p.add_extrude_multi(sid, vec![cid], 1.0, Reach::Forward, 0.0, vec![]);
            let body = p.finish_base_body(e, 1);
            p.set_feat_dim(body, "height", "h".into());
            Ok(body)
        })
        .expect("the block is laid");
    ctx
}

/// The volume of the one body standing on its own in an answer.
fn volume(reply: &Value) -> f64 {
    let bodies = reply["bodies"].as_array().unwrap_or_else(|| panic!("no bodies in {reply}"));
    assert_eq!(bodies.len(), 1, "{reply}");
    bodies[0]["volume"].as_f64().expect("a volume")
}

/// US-9: THE HEIGHT IS A PARAMETER, and changing it changes the part.
#[test]
fn a_changed_parameter_rebuilds_what_reads_it() {
    let mut ctx = block_of_height_h();
    let taller = call(&mut ctx, "set_parameter", json!({ "name": "H", "expr": "4 * 5" }));
    assert_eq!(taller["ok"], json!(true), "{taller}");
    assert_eq!(taller["parameter"]["name"], "h", "the name was taken as a new parameter, not as h in another case: {taller}");
    assert_eq!(taller["parameter"]["value"], json!(20.0), "{taller}");
    assert!((volume(&taller) - 24000.0).abs() < 1e-6, "h = 20 gives a block of {} mm^3", volume(&taller));
    assert_eq!(taller["red"], json!([]), "{taller}");

    let back = call(&mut ctx, "undo", json!({}));
    assert_eq!(back["undone"]["step"], "par-edit-step", "{back}");
    let listed = call(&mut ctx, "list_parameters", json!({}));
    assert_eq!(listed["parameters"], json!([{ "name": "h", "expr": "10", "value": 10.0 }]), "{listed}");
    let doc = call(&mut ctx, "get_document", json!({}));
    assert!((volume(&doc["document"]) - 12000.0).abs() < 1e-6, "undo did not take the block back: {doc}");
}

/// A TYPO IS REFUSED BEFORE THE DOCUMENT CHANGES: no step, no red feature, the old value kept.
#[test]
fn a_bad_expression_or_name_is_refused_before_the_change() {
    let mut ctx = block_of_height_h();
    let steps = ctx.doc.history().undo_names().len();
    let typo = call(&mut ctx, "set_parameter", json!({ "name": "h", "expr": "h *" }));
    assert_eq!(typo["ok"], json!(false), "{typo}");
    assert_eq!(typo["error"]["stage"], "validate");
    assert!(typo["error"]["code"].as_str().is_some_and(|c| c.starts_with("error-expr-")), "{typo}");
    let unknown = call(&mut ctx, "set_parameter", json!({ "name": "wall", "expr": "thickness / 2" }));
    assert_eq!(unknown["error"]["code"], "error-expr-unknown-name", "{unknown}");
    let itself = call(&mut ctx, "set_parameter", json!({ "name": "h", "expr": "h * 2" }));
    assert_eq!(itself["error"]["code"], "error-expr-unknown-name", "a parameter read itself: {itself}");
    let unfit = call(&mut ctx, "set_parameter", json!({ "name": "2w", "expr": "5" }));
    assert_eq!(unfit["error"]["code"], "unfit-name", "{unfit}");
    let missing = call(&mut ctx, "delete_parameter", json!({ "name": "nope" }));
    assert_eq!(missing["error"]["code"], "no-parameter", "{missing}");
    assert_eq!(ctx.doc.history().undo_names().len(), steps, "a refusal left a step");
    assert_eq!(ctx.doc.project().parameters[0].expr, "10");
}

/// A DELETED PARAMETER turns red what reads it, and the answer says so; undo brings both back.
#[test]
fn a_deleted_parameter_turns_red_what_reads_it() {
    let mut ctx = block_of_height_h();
    let gone = call(&mut ctx, "delete_parameter", json!({ "name": "h" }));
    assert_eq!(gone["ok"], json!(true), "{gone}");
    assert_eq!(gone["deleted"]["name"], "h");
    let red = gone["red"].as_array().expect("the red features");
    assert!(!red.is_empty(), "the block reads a parameter that is gone and nothing stands red: {gone}");
    let _ = call(&mut ctx, "undo", json!({}));
    let doc = call(&mut ctx, "get_document", json!({}));
    assert!(doc["document"]["features"].as_array().expect("features").iter().all(|f| f["error"].is_null()), "undo left a red feature: {doc}");
}
