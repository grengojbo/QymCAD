//! THE WHOLE CHAIN THROUGH THE SERVER, as a person asks it in a chat: "a bracket 60 x 40 x 4 with four holes for M3,
//! countersunk, its corners rounded, export a 3MF", then "make the thickness a parameter and rebuild it at 5".
//!
//! After EVERY step the document is checked whole - a chain breaks where a single tool does not: nothing stands red,
//! a feature that would stand red says why, a part stands as one body, every feature belongs to a part, no hole
//! drills the air. The failures of every step are gathered and told together.
//!
//! The chain run twice from nothing gives the same answers, word for word: the same call on the same document
//! answers the same.
mod check_folder;
use check_folder::CheckFolder;

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

/// What is wrong with the document after `step`, every fault named.
fn check_all(ctx: &mut Ctx, step: &str) -> Vec<String> {
    let mut wrong = Vec::new();
    let doc = call(ctx, "get_document", json!({}))["document"].clone();
    let features = doc["features"].as_array().cloned().unwrap_or_default();
    for f in &features {
        let error = &f["error"];
        if !error.is_null() {
            wrong.push(format!("{step}: feature {} ({}) stands red: {error}", f["key"], f["kind"]));
            if error["code"].as_str().is_none_or(str::is_empty) || error["message"].as_str().is_none_or(str::is_empty) {
                wrong.push(format!("{step}: feature {} stands red without a reason", f["key"]));
            }
        }
        if f["part"].is_null() {
            wrong.push(format!("{step}: feature {} ({}) belongs to no part", f["key"], f["kind"]));
        }
    }
    let bodies = doc["bodies"].as_array().cloned().unwrap_or_default();
    for p in doc["parts"].as_array().cloned().unwrap_or_default().iter().filter(|p| p["assembly"] == json!(false)) {
        let standing = bodies.iter().filter(|b| b["part"] == p["key"]).count();
        if standing > 1 {
            wrong.push(format!("{step}: part {} stands as {standing} bodies", p["key"]));
        }
    }
    wrong
}

/// The bracket: its parameter, its plate, its holes, its corners - each step's answer, and the faults found after it.
struct Run {
    answers: Vec<String>,
    wrong: Vec<String>,
    /// What the steps were asked to find, measured where they are asked.
    found: Vec<String>,
}

/// The vertical edges of a plate centred on the origin: where the front or the back meets the left or the right.
fn vertical_edges() -> Value {
    let side = |axis: &str, max: bool| json!({ "extreme": { "axis": axis, "max": max } });
    let corner = |x: bool, y: bool| json!({ "between": { "one": side("x", x), "other": side("y", y) } });
    json!({ "union": [corner(false, false), corner(false, true), corner(true, false), corner(true, true)] })
}

/// One step of the chain: what a person asks for, the tool, its arguments.
struct Step {
    what: &'static str,
    tool: &'static str,
    args: Value,
}

fn close(v: &Value, want: f64, by: f64) -> bool {
    v.as_f64().is_some_and(|x| (x - want).abs() < by)
}

fn run(dir: &std::path::Path) -> Run {
    let mut ctx = Ctx::blank();
    let mut r = Run { answers: Vec::new(), wrong: Vec::new(), found: Vec::new() };
    let file = dir.join("bracket.3mf");
    let steps = vec![
        Step { what: "a new document", tool: "new_project", args: json!({}) },
        Step { what: "the thickness as a parameter", tool: "set_parameter", args: json!({ "name": "t", "expr": "4" }) },
        Step { what: "a sketch on XY", tool: "create_sketch", args: json!({ "plane": "xy" }) },
        Step { what: "the plate's outline", tool: "sketch_add", args: json!({ "sketch": "$sketch", "entities": [{ "rect": { "from": [-30, -20], "to": [30, 20] } }] }) },
        Step { what: "the plate, t thick", tool: "extrude", args: json!({ "sketch": "$sketch", "distance": "t" }) },
        Step { what: "a sketch on its top", tool: "create_sketch", args: json!({ "plane": { "body": { "body": "$body" }, "face": "top" } }) },
        Step {
            what: "the centres of the holes, 8 in from the edges",
            tool: "sketch_add",
            args: json!({ "sketch": "$sketch", "entities": [
            { "point": { "at": [-22, -12] } }, { "point": { "at": [22, -12] } }, { "point": { "at": [-22, 12] } }, { "point": { "at": [22, 12] } },
        ] }),
        },
        Step {
            what: "four holes for M3, countersunk",
            tool: "hole",
            args: json!({ "sketch": "$sketch", "diameter": 3.4, "depth": 20, "kind": "countersink", "recess_diameter": 6.5, "recess_depth": 1.6 }),
        },
        Step { what: "the corners rounded 5", tool: "fillet", args: json!({ "edges": vertical_edges(), "radius": 5 }) },
        Step { what: "a look at it", tool: "render", args: json!({ "view": "iso", "width": 320, "height": 240 }) },
        Step { what: "the thickness made 5", tool: "set_parameter", args: json!({ "name": "t", "expr": "5" }) },
        Step { what: "a 3MF for the slicer", tool: "export_mesh", args: json!({ "path": file.to_string_lossy(), "quality": "standard" }) },
    ];
    let mut last = json!({});
    for Step { what, tool, args } in steps {
        // a step reads the sketch and the body the steps before it made
        let mut text = args.to_string();
        for key in ["sketch", "body"] {
            if let Some(v) = last.get(key).filter(|v| !v.is_null()) {
                text = text.replace(&format!("\"${key}\""), &v.to_string());
            }
        }
        let reply = call(&mut ctx, tool, serde_json::from_str(&text).expect("the arguments"));
        if reply["ok"] != json!(true) {
            r.wrong.push(format!("{what}: {tool} refused: {reply}"));
        }
        if reply.get("warnings").is_some() {
            r.wrong.push(format!("{what}: {tool} warned: {}", reply["warnings"]));
        }
        r.answers.push(reply.to_string().replace(&dir.to_string_lossy().to_string(), "<dir>"));
        r.wrong.extend(check_all(&mut ctx, what));
        for key in ["sketch", "body"] {
            if !reply[key].is_null() {
                last[key] = reply[key].clone();
            }
        }
    }

    // THE BRACKET AT 5: one sound body 5 high, its corners still rounded 5, its four holes still through
    let health = call(&mut ctx, "inspect", json!({}));
    r.found.push(format!("valid {} solids {} smallest radius {}", health["valid"], health["solids"], health["smallest_radius"]));
    if !(health["valid"] == json!(true) && health["solids"] == json!(1)) {
        r.wrong.push(format!("the bracket at 5 is not one sound body: {health}"));
    }
    // the smallest round face is the wall of a hole, 3.4 across; the corners are four walls of radius 5
    if !close(&health["smallest_radius"], 1.7, 1e-6) {
        r.wrong.push(format!("the smallest round face is not the wall of a hole: {health}"));
    }
    let faces = call(&mut ctx, "list_faces", json!({}))["faces"].as_array().cloned().unwrap_or_default();
    let corners = faces.iter().filter(|f| f["kind"] == "cylinder" && close(&f["radius"], 5.0, 1e-6)).count();
    r.found.push(format!("{corners} corners rounded 5"));
    if corners != 4 {
        r.wrong.push(format!("{corners} corners of the bracket at 5 are rounded 5, not 4"));
    }
    let doc = call(&mut ctx, "get_document", json!({}))["document"].clone();
    let top = &doc["bodies"][0]["max"][2];
    if !close(top, 5.0, 1e-9) {
        r.wrong.push(format!("the bracket is {top} high, not the 5 its parameter says"));
    }
    let edges = call(&mut ctx, "list_edges", json!({}))["edges"].as_array().cloned().unwrap_or_default();
    let rims_below = edges.iter().filter(|e| close(&e["circle"]["radius"], 1.7, 1e-6) && close(&e["middle"][2], 0.0, 1e-6)).count();
    r.found.push(format!("{rims_below} hole rims on the bottom"));
    if rims_below != 4 {
        r.wrong.push(format!("{rims_below} holes come out at the bottom of the bracket at 5, not 4"));
    }
    if std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0) == 0 {
        r.wrong.push(format!("no 3MF at {}", file.display()));
    }
    r
}

/// The folder of one chain in this run.
fn fresh_dir(name: &str) -> CheckFolder {
    CheckFolder::new(&format!("mcp-case-{name}"))
}

#[test]
fn a_bracket_is_made_and_rebuilt_at_a_new_thickness() {
    let r = run(fresh_dir("one").path());
    assert!(r.wrong.is_empty(), "the chain went wrong:\n{}\nfound: {:?}", r.wrong.join("\n"), r.found);
}

/// THE SAME CHAIN TWICE gives the same answers, word for word, and finds the same.
#[test]
fn the_chain_answers_the_same_twice() {
    let first = run(fresh_dir("first").path());
    let second = run(fresh_dir("second").path());
    let apart: Vec<String> = first.answers.iter().zip(&second.answers).enumerate().filter(|(_, (a, b))| a != b).map(|(i, (a, b))| format!("step {}:\n  {a}\n  {b}", i + 1)).collect();
    assert!(apart.is_empty(), "the answers differ:\n{}", apart.join("\n"));
    assert_eq!(first.found, second.found);
}
