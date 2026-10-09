//! RESOURCES, PROMPTS AND THE KERNEL'S WORDS through the server: the document and one feature read by address in the
//! JSON the tools answer with; an address with nothing at it refused with the protocol's code; the prompts listed and
//! filled with the person's words; a feature the kernel would not build answered with the kernel's own words beside
//! the coded reason, and a call that went through without them.

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

/// The whole reply to a request.
fn ask(ctx: &mut Ctx, method: &str, params: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }).to_string();
    answer(ctx, &line).expect("a request is answered")
}

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    ask(ctx, "tools/call", json!({ "name": name, "arguments": arguments }))["result"]["structuredContent"].clone()
}

fn ok(reply: Value) -> Value {
    assert_eq!(reply["ok"], json!(true), "{reply}");
    reply
}

/// The one text a resource read carries.
fn text(ctx: &mut Ctx, uri: &str) -> String {
    let r = ask(ctx, "resources/read", json!({ "uri": uri }));
    let contents = &r["result"]["contents"][0];
    assert_eq!(contents["uri"], json!(uri), "{r}");
    assert_eq!(contents["mimeType"], "application/json", "{r}");
    contents["text"].as_str().unwrap_or_else(|| panic!("no text in {r}")).to_string()
}

/// That text read as JSON - for its fields; a number read back may differ in its last digit, so whole documents are
/// compared as text.
fn read(ctx: &mut Ctx, uri: &str) -> Value {
    serde_json::from_str(&text(ctx, uri)).expect("the text is JSON")
}

#[test]
fn the_document_and_a_feature_are_read_by_address() {
    let mut ctx = Ctx::blank();
    let listed = ask(&mut ctx, "resources/list", json!({}));
    assert_eq!(listed["result"]["resources"][0]["uri"], "qymcad://document", "{listed}");
    let templates = ask(&mut ctx, "resources/templates/list", json!({}));
    assert_eq!(templates["result"]["resourceTemplates"][0]["uriTemplate"], "qymcad://feature/{key}", "{templates}");

    let _ = ok(call(&mut ctx, "box", json!({ "x": 40, "y": 30, "z": 10 })));
    let round = ok(call(&mut ctx, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": 2 })));
    let doc = text(&mut ctx, "qymcad://document");
    assert_eq!(doc, ok(call(&mut ctx, "get_document", json!({})))["document"].to_string(), "the resource is not what get_document answers");
    let key = round["feature"].as_u64().expect("the fillet's key");
    let feature = read(&mut ctx, &format!("qymcad://feature/{key}"));
    assert_eq!(feature["kind"], "Fillet", "{feature}");
    let sizes = feature["sizes"].as_array().expect("sizes");
    assert!(sizes.iter().any(|s| s["value"] == json!(2.0)), "the radius is not among the sizes: {feature}");
}

#[test]
fn an_address_with_nothing_at_it_is_refused_with_the_protocols_code() {
    let mut ctx = Ctx::blank();
    for (uri, code) in [("qymcad://feature/999", -32002), ("qymcad://nothing", -32002), ("qymcad://feature/top", -32602)] {
        let r = ask(&mut ctx, "resources/read", json!({ "uri": uri }));
        assert_eq!(r["error"]["code"], json!(code), "{uri}: {r}");
    }
}

#[test]
fn the_prompts_are_listed_and_filled_with_the_persons_words() {
    let mut ctx = Ctx::blank();
    let listed = ask(&mut ctx, "prompts/list", json!({}));
    let names: Vec<&str> = listed["result"]["prompts"].as_array().expect("prompts").iter().filter_map(|p| p["name"].as_str()).collect();
    assert_eq!(names, ["design_for_fdm", "edit_stl", "edit_step", "edit_what_i_selected"], "{listed}");

    let got = ask(&mut ctx, "prompts/get", json!({ "name": "design_for_fdm", "arguments": { "part": "a bracket 60 x 40 x 4", "nozzle": "0.6" } }));
    let text = got["result"]["messages"][0]["content"]["text"].as_str().unwrap_or_else(|| panic!("no text in {got}"));
    assert!(text.contains("a bracket 60 x 40 x 4"), "the person's words are not in the prompt: {text}");
    assert!(text.contains("1.8 mm"), "the walls are not three lines of a 0.6 nozzle: {text}");
    assert_eq!(got["result"]["messages"][0]["role"], "user", "{got}");

    let bare = ask(&mut ctx, "prompts/get", json!({ "name": "edit_step", "arguments": { "path": "a.step" } }));
    assert_eq!(bare["error"]["code"], json!(-32602), "a prompt without its required change went: {bare}");
    let none = ask(&mut ctx, "prompts/get", json!({ "name": "no_such_prompt" }));
    assert_eq!(none["error"]["code"], json!(-32602), "{none}");
}

/// THE KERNEL'S OWN WORDS go beside a feature left red, and nowhere else: not on the next call, which went through.
#[test]
fn a_feature_the_kernel_would_not_build_carries_its_words() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 40, "y": 30, "z": 10 })));
    let red = ok(call(&mut ctx, "fillet", json!({ "edges": "all", "radius": 30 })));
    assert!(!red["red"].as_array().expect("red").is_empty(), "a rounding of 30 on a block 10 high stood: {red}");
    let said = red["kernel_said"].as_str().unwrap_or_else(|| panic!("the kernel's words are missing: {red}"));
    assert!(said.starts_with("fillet/"), "the words do not name the place in the kernel: {said}");

    // a refusal of the server's own right after it - a radius below nothing - is not the kernel's, and its words
    // are not told there
    let refused = call(&mut ctx, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": -1 }));
    assert_eq!(refused["ok"], json!(false), "{refused}");
    assert!(refused.get("kernel_said").is_none(), "the kernel's old words are told on a refusal of another reason: {refused}");

    let _ = ok(call(&mut ctx, "undo", json!({})));
    let fine = ok(call(&mut ctx, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": 1 })));
    assert!(fine.get("kernel_said").is_none(), "an old refusal is told on a call that went through: {fine}");
}
