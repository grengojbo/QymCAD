//! THE DOCUMENT AS A WHOLE through the server: a new one, written out and opened again with the same bodies, a file
//! that is not there refused with the file layer's code, and no file written over that the call did not name.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

fn out_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-doc")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

/// The bodies standing on their own, each as its volume to the micro-cubic millimetre and its faces.
fn bodies(document: &Value) -> Vec<String> {
    let mut out: Vec<String> = document["bodies"]
        .as_array()
        .expect("the bodies are a list")
        .iter()
        .map(|b| format!("{:.6} mm^3, {} faces, {} edges", b["volume"].as_f64().expect("a volume"), b["faces"], b["edges"]))
        .collect();
    out.sort();
    out
}

#[test]
fn a_new_document_starts_as_asked() {
    let mut ctx = Ctx::blank();
    let part = call(&mut ctx, "new_project", json!({}));
    assert_eq!(part["ok"], json!(true), "{part}");
    assert_eq!(part["document"]["parts"].as_array().map(Vec::len), Some(1), "the default document has one part: {part}");
    let empty = call(&mut ctx, "new_project", json!({ "template": "empty" }));
    assert_eq!(empty["document"]["parts"].as_array().map(Vec::len), Some(0), "the empty document has a part: {empty}");
    let wrong = call(&mut ctx, "new_project", json!({ "template": "drawing" }));
    assert_eq!(wrong["error"]["stage"], "validate", "a template there is none of went through: {wrong}");
}

/// WRITTEN OUT AND OPENED AGAIN, the document holds the same bodies, and the history starts over.
#[test]
fn a_saved_document_opens_with_the_same_bodies() {
    let dir = out_dir("round");
    let file = dir.join("box.qcad");
    let mut ctx = Ctx::blank();
    let _ = ctx.doc.edit("cmd-box", |p| Ok(p.add_box(10.0, 20.0, 30.0))).expect("the box is laid");
    let before = call(&mut ctx, "get_document", json!({}));
    assert_eq!(bodies(&before["document"]), ["6000.000000 mm^3, 6 faces, 12 edges"], "{before}");

    let saved = call(&mut ctx, "save_project", json!({ "path": file.to_string_lossy() }));
    assert_eq!(saved["ok"], json!(true), "{saved}");
    assert_eq!(saved["path"], json!(file.to_string_lossy()), "the answer does not name the file written");
    assert!(file.is_file(), "nothing was written");
    // saved again to its own file: no leave to write over it is needed
    let again = call(&mut ctx, "save_project", json!({}));
    assert_eq!(again["ok"], json!(true), "the document's own file was refused: {again}");

    let _ = call(&mut ctx, "new_project", json!({}));
    let opened = call(&mut ctx, "open_project", json!({ "path": file.to_string_lossy() }));
    assert_eq!(opened["ok"], json!(true), "{opened}");
    assert_eq!(bodies(&opened["document"]), bodies(&before["document"]), "the document came back other than it went");
    assert_eq!(opened["document"]["path"], json!(file.to_string_lossy()));
    assert_eq!(opened["document"]["undo"], json!([]), "an opened document has a history");
}

#[test]
fn a_file_that_is_not_there_is_refused_with_its_code() {
    let mut ctx = Ctx::blank();
    let path = out_dir("missing").join("nothing.qcad");
    let r = call(&mut ctx, "open_project", json!({ "path": path.to_string_lossy() }));
    assert_eq!(r["ok"], json!(false), "{r}");
    assert_eq!(r["error"]["code"], "io-file-read", "{r}");
    assert_eq!(r["error"]["stage"], "io");
    assert!(r["error"]["message"].as_str().is_some_and(|m| m.starts_with("could not read")), "the message is not the catalogue's English: {r}");
    let wrong = call(&mut ctx, "open_project", json!({ "path": "model.stl" }));
    assert_eq!(wrong["error"]["code"], "wrong-extension", "{wrong}");
}

/// A FILE THAT IS THERE is written over only when the call says so; a document with no file asks for a path.
#[test]
fn no_file_is_written_over_unasked() {
    let dir = out_dir("guard");
    let file = dir.join("someone.qcad");
    std::fs::write(&file, b"a person's work").expect("a file to guard");
    let mut ctx = Ctx::blank();
    let nowhere = call(&mut ctx, "save_project", json!({}));
    assert_eq!(nowhere["error"]["code"], "no-path", "{nowhere}");
    let kept = call(&mut ctx, "save_project", json!({ "path": file.to_string_lossy() }));
    assert_eq!(kept["error"]["code"], "path-taken", "{kept}");
    assert_eq!(std::fs::read(&file).expect("the file reads"), b"a person's work", "the file was written over");
    let over = call(&mut ctx, "save_project", json!({ "path": file.to_string_lossy(), "overwrite": true }));
    assert_eq!(over["ok"], json!(true), "{over}");
}

/// THE ACCOUNT names a red feature with its code and words, and `full` adds the bodies taken into another one.
#[test]
fn the_account_tells_what_stands_red() {
    let mut ctx = Ctx::blank();
    let body = ctx.doc.edit("cmd-box", |p| Ok(p.add_box(10.0, 10.0, 10.0))).expect("the box is laid");
    use qymcad_core::refs::{Axis, Query, Ref};
    let rim = Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })));
    let _ = ctx.doc.edit("cmd-fillet", |p| Ok(p.add_fillet_ref(body, 50.0, rim))).expect("the fillet is laid, red");
    let summary = call(&mut ctx, "get_document", json!({}));
    let red: Vec<&Value> = summary["document"]["features"].as_array().expect("features").iter().filter(|f| !f["error"].is_null()).collect();
    assert_eq!(red.len(), 1, "one feature stands red: {summary}");
    assert!(red[0]["error"]["code"].as_str().is_some_and(|c| c.starts_with("error-")), "{}", red[0]);
    assert!(red[0]["error"]["message"].as_str().is_some_and(|m| !m.starts_with("error-")), "the error has no words: {}", red[0]);
    let full = call(&mut ctx, "get_document", json!({ "detail": "full" }));
    assert!(full["document"]["bodies"].as_array().map(Vec::len) >= summary["document"]["bodies"].as_array().map(Vec::len));
}
