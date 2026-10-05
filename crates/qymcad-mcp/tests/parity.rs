//! THE SERVER LAYS WHAT THE WINDOW LAYS. Every tool of the window that has a twin here is described by a contract
//! (`qymcad_acceptance::tools`): its fields, the body its typical values give, and the body each field gives when it
//! alone changes. The twin is called with the same numbers and its body is held to the same account, with the same
//! allowances - so a primitive that drifts from the window turns this red, whichever side moved.

use qymcad_acceptance::contract::{Class, Outcome, Tool};
use qymcad_acceptance::tools::primitives;
use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Map, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone()
}

/// A TOOL OF THE WINDOW AND ITS TWIN HERE: the twin's name and the argument each field of the contract goes to, in
/// the contract's order.
struct Twin {
    contract: &'static Tool,
    name: &'static str,
    args: &'static [&'static str],
}

const TWINS: &[Twin] = &[
    Twin { contract: &primitives::BOX, name: "box", args: &["x", "y", "z"] },
    Twin { contract: &primitives::CYLINDER, name: "cylinder", args: &["radius", "height"] },
    Twin { contract: &primitives::SPHERE, name: "sphere", args: &["radius"] },
    Twin { contract: &primitives::CONE, name: "cone", args: &["bottom_radius", "top_radius", "height"] },
    Twin { contract: &primitives::TORUS, name: "torus", args: &["ring_radius", "tube_radius"] },
    Twin { contract: &primitives::PRISM, name: "prism", args: &["radius", "height", "sides"] },
];

/// The arguments of `twin` at the typical values, one field (by its place) set to `value`.
fn arguments(twin: &Twin, changed: Option<Change>) -> Value {
    let mut a = Map::new();
    for (i, f) in twin.contract.fields.iter().enumerate() {
        let v = changed.filter(|c| c.field == i).map_or(f.typical, |c| c.value);
        let v = if f.class == Class::Count { json!(v.round() as u64) } else { json!(v) };
        a.insert(twin.args[i].to_string(), v);
    }
    Value::Object(a)
}

/// One field of a contract set away from its typical value.
#[derive(Clone, Copy)]
struct Change {
    field: usize,
    value: f64,
}

/// HOLD THE ANSWER TO THE ACCOUNT, as the contract run does: the one body standing, volume to 1e-5 of itself, faces and
/// edges exact, its box to 0.0015 of its diagonal (the mesh lies inside a curve by up to the deflection), at least
/// 0.05 mm.
fn held(reply: &Value, want: &Outcome) -> Result<(), String> {
    if reply["ok"] != json!(true) {
        return Err(format!("refused: {reply}"));
    }
    let Outcome::Body { volume, faces, edges, min, max } = *want else { return Err(format!("the contract wants {want:?}, not a body")) };
    let bodies = reply["bodies"].as_array().ok_or("no bodies in the answer")?;
    let [b] = bodies.as_slice() else { return Err(format!("{} bodies stand, not one", bodies.len())) };
    let num = |v: &Value| v.as_f64().unwrap_or(f64::NAN);
    let point = |v: &Value| [num(&v[0]), num(&v[1]), num(&v[2])];
    let diagonal = (0..3).map(|i| (max[i] - min[i]).powi(2)).sum::<f64>().sqrt();
    let slack = (0.0015 * diagonal).max(0.05);
    let near = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() <= slack);
    let got_volume = num(&b["volume"]);
    let ok = (got_volume - volume).abs() <= 1e-5 * volume.abs().max(1.0) && b["faces"] == json!(faces) && b["edges"] == json!(edges) && near(point(&b["min"]), min) && near(point(&b["max"]), max);
    if ok {
        Ok(())
    } else {
        Err(format!("the body is volume {got_volume} faces {} edges {} min {} max {}, the window's is {want:?}", b["faces"], b["edges"], b["min"], b["max"]))
    }
}

/// A value away from the typical one and inside the field's range: half as much again, or 5 where the typical is 0.
fn away(typical: f64, lo: f64, hi: f64) -> f64 {
    let v = if typical > 0.0 { typical * 1.5 } else { 5.0 };
    v.clamp(lo, hi)
}

/// EVERY TWIN at its typical values gives the contract's body, and every field changed alone gives the body the
/// contract says that field gives. The misses are reported together.
#[test]
fn a_primitive_is_the_body_the_window_lays() {
    let mut misses = Vec::new();
    for twin in TWINS {
        assert_eq!(twin.args.len(), twin.contract.fields.len(), "{}: the arguments do not follow the fields of {}", twin.name, twin.contract.id);
        let mut ctx = Ctx::blank();
        let typical = call(&mut ctx, twin.name, arguments(twin, None));
        if let Err(e) = held(&typical, &twin.contract.result) {
            misses.push(format!("{} typical: {e}", twin.name));
        }
        for (i, f) in twin.contract.fields.iter().enumerate() {
            let change = Change { field: i, value: away(f.typical, f.lo, f.hi) };
            let mut ctx = Ctx::blank();
            let reply = call(&mut ctx, twin.name, arguments(twin, Some(change)));
            if let Err(e) = held(&reply, &(f.outcome)(change.value)) {
                misses.push(format!("{} {} = {}: {e}", twin.name, twin.args[i], change.value));
            }
        }
    }
    assert!(misses.is_empty(), "the server and the window differ:\n{}", misses.join("\n"));
}

/// EVERY PRIMITIVE OF THE WINDOW has its twin here: a primitive added to the window and not to the server is noticed.
#[test]
fn every_primitive_of_the_window_has_a_twin() {
    let window: Vec<&str> = qymcad_acceptance::tools::ALL.iter().filter(|t| t.help == "part/19-primitives").map(|t| t.id).collect();
    let twinned: Vec<&str> = TWINS.iter().map(|t| t.contract.id).collect();
    let alone: Vec<&&str> = window.iter().filter(|id| !twinned.contains(id)).collect();
    assert!(alone.is_empty(), "primitives of the window with no twin: {alone:?}");
    assert!(window.len() >= 6, "the primitives are no longer found by their help article: {window:?}");
}

fn volume(reply: &Value) -> f64 {
    reply["bodies"][0]["volume"].as_f64().unwrap_or_else(|| panic!("no volume in {reply}"))
}

/// A SIZE GIVEN AS AN EXPRESSION stays one: the box follows its parameter, and the box is one undo step.
#[test]
fn a_size_given_as_an_expression_follows_its_parameter() {
    let mut ctx = Ctx::blank();
    let _ = call(&mut ctx, "set_parameter", json!({ "name": "w", "expr": "30" }));
    let laid = call(&mut ctx, "box", json!({ "x": "w", "y": 20, "z": "w / 3" }));
    assert!((volume(&laid) - 30.0 * 20.0 * 10.0).abs() < 1e-6, "{laid}");
    let moved = call(&mut ctx, "set_parameter", json!({ "name": "w", "expr": "60" }));
    assert!((volume(&moved) - 60.0 * 20.0 * 20.0).abs() < 1e-6, "the box did not follow w: {moved}");
    let _ = call(&mut ctx, "undo", json!({}));
    let undone = call(&mut ctx, "undo", json!({}));
    assert_eq!(undone["undone"]["step"], "cmd-box", "the box is not one step: {undone}");
}

/// A PRIMITIVE TURNED AND CARRIED: a cylinder turned a quarter about X lies along -Y, and `at` carries it up.
#[test]
fn a_primitive_is_turned_then_carried() {
    let mut ctx = Ctx::blank();
    let laid = call(&mut ctx, "cylinder", json!({ "radius": 5, "height": 40, "rotate": [{ "axis": [1, 0, 0], "degrees": 90 }], "at": [0, 0, 30] }));
    let b = &laid["bodies"][0];
    let at = |k: &str, i: usize| b[k][i].as_f64().expect("a box");
    assert!((at("min", 1) + 40.0).abs() < 0.1 && at("max", 1).abs() < 0.1, "the cylinder does not lie along -Y: {b}");
    assert!((at("min", 2) - 25.0).abs() < 0.1 && (at("max", 2) - 35.0).abs() < 0.1, "the cylinder was not carried to Z 30: {b}");
    let bad = call(&mut ctx, "sphere", json!({ "radius": 5, "rotate": [{ "axis": [0, 0, 0], "degrees": 90 }] }));
    assert_eq!(bad["error"]["code"], "bad-axis", "{bad}");
}

/// A SECOND PRIMITIVE JOINS THE PART'S BODY: a part is one body.
#[test]
fn a_second_primitive_joins_the_body() {
    let mut ctx = Ctx::blank();
    let _ = call(&mut ctx, "box", json!({ "x": 20, "y": 20, "z": 10 }));
    let joined = call(&mut ctx, "cylinder", json!({ "radius": 5, "height": 10, "at": [0, 0, 10] }));
    assert_eq!(joined["bodies"].as_array().map(Vec::len), Some(1), "the part stands as two bodies: {joined}");
    let want = 4000.0 + std::f64::consts::PI * 25.0 * 10.0;
    assert!((volume(&joined) - want).abs() < 1e-3, "{joined}");
}

/// A call and the code it is refused with.
struct Refused {
    tool: &'static str,
    arguments: Value,
    code: &'static str,
}

/// WHAT CANNOT BE BUILT is refused before the document changes: a size of nothing, a tube as thick as its ring, two
/// sides, a name no parameter has.
#[test]
fn what_cannot_be_built_is_refused() {
    let mut ctx = Ctx::blank();
    let cases = [
        Refused { tool: "box", arguments: json!({ "x": 0, "y": 10, "z": 10 }), code: "bad-size" },
        Refused { tool: "cone", arguments: json!({ "bottom_radius": 10, "top_radius": -1, "height": 5 }), code: "bad-size" },
        Refused { tool: "torus", arguments: json!({ "ring_radius": 4, "tube_radius": 4 }), code: "tube-too-thick" },
        Refused { tool: "prism", arguments: json!({ "radius": 10, "height": 5, "sides": 2 }), code: "bad-size" },
        Refused { tool: "sphere", arguments: json!({ "radius": "nothing_by_this_name" }), code: "error-expr-unknown-name" },
    ];
    for c in cases {
        let reply = call(&mut ctx, c.tool, c.arguments);
        assert_eq!(reply["error"]["code"], c.code, "{}: {reply}", c.tool);
        assert_eq!(reply["error"]["stage"], "validate", "{}: {reply}", c.tool);
    }
    assert!(ctx.doc.history().undo_names().is_empty(), "a refusal left a step");
}
