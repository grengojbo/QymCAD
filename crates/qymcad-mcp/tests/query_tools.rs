//! ASKING THE MODEL through the server, on parts whose numbers are known: a block 40 x 30 x 10 has six flat faces of
//! 1200, 400 and 300 mm^2; a cylinder 10 round has one cylindrical wall of radius 10 and two flat ends; a cone's
//! wall is turned; a query finds what a feature would take, and refuses where a feature would; a body is sound, one
//! solid, and a rounding of 2 is its smallest; distances and angles come out as the window's measuring tool gives
//! them; every place is in the world, so a block moved 100 along X is found there.

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

/// The block 40 x 30 x 10: centred on the origin in X and Y, standing on Z = 0.
fn block() -> Ctx {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 40, "y": 30, "z": 10 })));
    ctx
}

fn close(v: &Value, want: f64) -> bool {
    v.as_f64().is_some_and(|x| (x - want).abs() < 1e-6)
}

fn point(v: &Value) -> [f64; 3] {
    let a = v.as_array().unwrap_or_else(|| panic!("not a point: {v}"));
    [a[0].as_f64().expect("x"), a[1].as_f64().expect("y"), a[2].as_f64().expect("z")]
}

#[test]
fn the_faces_of_a_block_and_of_a_cylinder_are_listed_with_their_kinds() {
    let mut ctx = block();
    let faces = ok(call(&mut ctx, "list_faces", json!({})))["faces"].as_array().cloned().expect("faces");
    assert_eq!(faces.len(), 6, "{faces:?}");
    assert!(faces.iter().all(|f| f["kind"] == "plane"), "a face of the block is not flat: {faces:?}");
    let mut areas: Vec<f64> = faces.iter().map(|f| f["area"].as_f64().expect("area")).collect();
    areas.sort_by(f64::total_cmp);
    assert_eq!(areas, vec![300.0, 300.0, 400.0, 400.0, 1200.0, 1200.0], "the areas of 40 x 30 x 10");
    let top = faces.iter().find(|f| f["normal"] == json!([0.0, 0.0, 1.0])).expect("a face facing up");
    assert_eq!(point(&top["centre"]), [0.0, 0.0, 10.0], "{top}");

    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "cylinder", json!({ "radius": 10, "height": 20 })));
    let faces = ok(call(&mut ctx, "list_faces", json!({})))["faces"].as_array().cloned().expect("faces");
    let walls: Vec<&Value> = faces.iter().filter(|f| f["kind"] == "cylinder").collect();
    assert_eq!(walls.len(), 1, "{faces:?}");
    assert!(close(&walls[0]["radius"], 10.0), "{}", walls[0]);
    let axis = point(&walls[0]["axis"]["dir"]);
    assert!((axis[2].abs() - 1.0).abs() < 1e-6, "the wall's axis is not upright: {axis:?}");
    assert_eq!(faces.iter().filter(|f| f["kind"] == "plane").count(), 2, "{faces:?}");

    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "cone", json!({ "bottom_radius": 10, "top_radius": 4, "height": 12 })));
    let faces = ok(call(&mut ctx, "list_faces", json!({})))["faces"].as_array().cloned().expect("faces");
    assert_eq!(faces.iter().filter(|f| f["kind"] == "turned").count(), 1, "the cone's wall is not a turned surface: {faces:?}");
}

#[test]
fn the_edges_of_a_tube_have_their_circles() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "cylinder", json!({ "radius": 12, "height": 30 })));
    let _ = ok(call(&mut ctx, "hole", json!({ "face": "top", "diameter": 14, "depth": 40 })));
    let edges = ok(call(&mut ctx, "list_edges", json!({})))["edges"].as_array().cloned().expect("edges");
    let mut radii: Vec<f64> = edges.iter().filter_map(|e| e["circle"]["radius"].as_f64()).collect();
    radii.sort_by(f64::total_cmp);
    assert_eq!(radii, vec![7.0, 7.0, 12.0, 12.0], "the rims of a tube 24 across with a 14 bore: {edges:?}");
}

#[test]
fn a_query_finds_what_a_feature_would_take() {
    let mut ctx = block();
    let top = ok(call(&mut ctx, "resolve", json!({ "faces": "top" })));
    assert_eq!(top["count"], 1, "{top}");
    let rim = ok(call(&mut ctx, "resolve", json!({ "edges": { "adjacent": "top" } })));
    assert_eq!(rim["count"], 4, "{rim}");
    // a feature asking for one edge of the four is refused, and so is this
    let one = call(&mut ctx, "resolve", json!({ "edges": { "adjacent": "top" }, "expect": "one" }));
    assert_eq!(one["ok"], json!(false), "{one}");
    let both = call(&mut ctx, "resolve", json!({ "faces": "top", "edges": "top" }));
    assert_eq!(both["error"]["code"], "arguments", "{both}");
}

#[test]
fn a_sound_body_is_one_solid_and_its_smallest_radius_is_named() {
    let mut ctx = block();
    let bare = ok(call(&mut ctx, "inspect", json!({})));
    assert_eq!(bare["valid"], json!(true), "{bare}");
    assert_eq!(bare["solids"], json!(1), "{bare}");
    assert_eq!(bare["surfaces"], json!({ "plane": 6 }), "{bare}");
    assert_eq!(bare["smallest_radius"], Value::Null, "a block with no round face has a radius: {bare}");
    let _ = ok(call(&mut ctx, "fillet", json!({ "edges": { "adjacent": "top" }, "radius": 2 })));
    let rounded = ok(call(&mut ctx, "inspect", json!({})));
    assert!(close(&rounded["smallest_radius"], 2.0), "{rounded}");
    assert!(rounded["surfaces"]["cylinder"].as_u64().is_some_and(|n| n >= 4), "the rounded rim has no cylinders: {rounded}");
}

#[test]
fn measurements_come_out_as_the_window_gives_them() {
    let mut ctx = block();
    // two parallel faces: their distance, at no angle
    let height = ok(call(&mut ctx, "measure", json!({ "a": { "face": "top" }, "b": { "face": "bottom" } })));
    assert!(close(&height["distance"], 10.0) && close(&height["angle"], 0.0), "{height}");
    // two faces at a right angle: the angle, and no distance - it is not one number
    let corner = ok(call(&mut ctx, "measure", json!({ "a": { "face": "top" }, "b": { "face": "front" } })));
    assert!(close(&corner["angle"], 90.0) && corner.get("distance").is_none(), "{corner}");
    // two points: the distance and its parts along the axes
    let apart = ok(call(&mut ctx, "measure", json!({ "a": { "point": [0, 0, 0] }, "b": { "point": [3, 4, 0] } })));
    assert!(close(&apart["distance"], 5.0), "{apart}");
    assert_eq!(apart["delta"], json!([3.0, 4.0, 0.0]), "{apart}");
    // one edge of the top along X: its length
    let long = ok(call(&mut ctx, "measure", json!({ "a": { "edge": { "between": { "one": "top", "other": "front" } } } })));
    assert!(close(&long["length"], 40.0), "{long}");

    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "cylinder", json!({ "radius": 12, "height": 30 })));
    let _ = ok(call(&mut ctx, "hole", json!({ "face": "top", "diameter": 14, "depth": 40 })));
    let found = ok(call(&mut ctx, "list_faces", json!({})))["faces"].as_array().cloned().expect("faces");
    let wall = found.iter().find(|f| close(&f["radius"], 7.0)).expect("the bore's wall");
    let d = ok(call(&mut ctx, "measure", json!({ "a": { "face": { "ids": [wall["key"]] } } })));
    assert!(close(&d["diameter"], 14.0), "the bore is not 14 across: {d}");
}

/// EVERY PLACE IS IN THE WORLD. A part placed in an assembly keeps its bodies in its own frame and stands where its
/// component puts it - here 100 along X, as a document with an assembly opens; a `move` would bake the shift into the
/// body and prove nothing about the frame.
#[test]
fn every_place_is_in_the_world() {
    let mut ctx = Ctx::blank();
    let laid = ok(call(&mut ctx, "box", json!({ "x": 40, "y": 30, "z": 10 })));
    let part = laid["bodies"][0]["part"].as_u64().expect("the part") as qymcad_core::model::Id;
    ctx.doc
        .edit("f-operation", |p| {
            p.set_component_transform(part, [1.0, 0.0, 0.0, 100.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
            Ok(())
        })
        .expect("the part is placed");
    let faces = ok(call(&mut ctx, "list_faces", json!({ "body": { "part": part } })))["faces"].as_array().cloned().expect("faces");
    let top = faces.iter().find(|f| f["normal"] == json!([0.0, 0.0, 1.0])).expect("a face facing up");
    assert_eq!(point(&top["centre"]), [100.0, 0.0, 10.0], "the placed block's top is not where it stands: {top}");
    let below = ok(call(&mut ctx, "measure", json!({ "a": { "point": [100, 0, 0] }, "b": { "body": { "part": part }, "face": "top" } })));
    assert!(close(&below["distance"], 10.0), "{below}");
    let edges = ok(call(&mut ctx, "list_edges", json!({ "body": { "part": part } })))["edges"].as_array().cloned().expect("edges");
    assert!(edges.iter().all(|e| point(&e["middle"])[0] >= 80.0 - 1e-6), "an edge of the placed block stands near the origin: {edges:?}");
}

#[test]
fn an_end_that_is_not_one_thing_is_refused() {
    let mut ctx = block();
    let many = call(&mut ctx, "measure", json!({ "a": { "face": "all" } }));
    assert_eq!(many["ok"], json!(false), "a query finding six faces was measured: {many}");
    let two = call(&mut ctx, "measure", json!({ "a": { "face": "top", "point": [0, 0, 0] } }));
    assert_eq!(two["error"]["code"], "arguments", "{two}");
    let nothing = call(&mut ctx, "list_faces", json!({ "body": { "body": 999 } }));
    assert_eq!(nothing["error"]["code"], "no-body", "{nothing}");
}

/// WHAT IS SELECTED IS KNOWN ONLY IN A WINDOW: the program on a document of its own says so, with the way to a window,
/// rather than an empty list that would read as "nothing is selected".
#[test]
fn the_selection_without_a_window_is_refused() {
    let mut ctx = block();
    let reply = call(&mut ctx, "get_selection", json!({}));
    assert_eq!(reply["ok"], json!(false), "{reply}");
    assert_eq!(reply["error"]["code"], "no-window", "{reply}");
    assert_eq!(reply["error"]["stage"], "window", "{reply}");
}
