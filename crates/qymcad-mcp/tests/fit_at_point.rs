//! WHAT `recognise_feature` WOULD MAKE AT A POINT, ASKED WITHOUT MAKING IT: `fit_at_point` reads the hole, rounding or
//! chamfer at a point of a body - its kind, the size measured and the size it would be laid at, the depth of a hole
//! and whether it runs through - and changes nothing. What it answers is what `recognise_feature` then lays.
//!
//! And the way from a picture to a feature, by eye: the hole seen in the picture `render` draws, its wall found under
//! a pixel by `point_at`, read by `fit_at_point`, made editable by `recognise_feature`.

use qymcad_mcp::picture::{self, Edges, Lit, Look, Size, View};
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

/// A document and the key of its body, as the tools take it.
struct Drilled {
    ctx: Ctx,
    body: Value,
}

/// A block 40 x 30 x 10 standing on the origin, drilled 10 across through its middle.
fn drilled() -> Drilled {
    let mut ctx = Ctx::blank();
    let _ = ctx.doc.edit("cmd-box", |p| Ok(p.add_box(40.0, 30.0, 10.0))).expect("the block is laid");
    let made = ok(call(&mut ctx, "hole", json!({ "face": "top", "diameter": 10, "depth": 20 })));
    let body = json!({ "body": made["bodies"][0]["id"] });
    Drilled { ctx, body }
}

/// How many steps the document has been through: a read leaves it where it was.
fn steps(ctx: &mut Ctx) -> Value {
    let doc = ok(call(ctx, "get_document", json!({})));
    json!({ "features": doc["document"]["features"], "undo": doc["document"]["undo"] })
}

/// The point on the wall of the hole, read: a hole of 10 through, laid at 10; the document as it was; and
/// `recognise_feature` at the same point finds the same.
#[test]
fn the_hole_at_a_point_is_read_and_nothing_is_laid() {
    let Drilled { mut ctx, body } = drilled();
    let before = steps(&mut ctx);
    let read = ok(call(&mut ctx, "fit_at_point", json!({ "body": body, "at": [0.0, 5.0, 5.0] })));
    assert_eq!(steps(&mut ctx), before, "a read changed the document: {read}");
    let found = &read["found"];
    assert_eq!(found["kind"], "hole", "{read}");
    assert_eq!(found["through"], json!(true), "{read}");
    assert!((found["measured"].as_f64().unwrap_or(0.0) - 10.0).abs() < 0.05, "{read}");
    assert_eq!(found["size"], json!(10.0), "{read}");
    let made = ok(call(&mut ctx, "recognise_feature", json!({ "body": body, "at": [0.0, 5.0, 5.0] })));
    for key in ["kind", "measured", "through"] {
        assert_eq!(made["found"][key], found[key], "recognise_feature found another {key}: {made} after {read}");
    }
}

/// A plain face holds no feature: refused with the code `recognise_feature` gives it.
#[test]
fn a_plain_face_is_refused_as_recognise_feature_refuses_it() {
    let Drilled { mut ctx, body } = drilled();
    let read = call(&mut ctx, "fit_at_point", json!({ "body": body, "at": [-20.0, 0.0, 5.0] }));
    assert_eq!(read["error"]["code"], "no-feature", "{read}");
}

/// BY EYE: the far wall of the hole, as the iso view shows it, carried to its pixel; `point_at` there gives a round
/// face and the point to read; `fit_at_point` reads a hole of 10; `recognise_feature` makes it.
#[test]
fn a_hole_seen_in_the_picture_is_made_editable() {
    let Drilled { mut ctx, .. } = drilled();
    let look = Look { view: View::Iso, size: Size { width: 800, height: 600 }, lit: Lit::Nothing, edges: Edges::Drawn, window: None };
    // the iso eye stands at +X -Y +Z: the wall it looks into is the one at -X +Y
    let r = 5.0 / std::f64::consts::SQRT_2;
    let px = picture::pixel_of(ctx.doc.project(), &look, [-r, r, 8.0]).expect("the wall is in the picture");
    let hit = ok(call(&mut ctx, "point_at", json!({ "view": "iso", "x": px.x, "y": px.y })));
    assert!(hit["face"]["radius"].as_f64().is_some_and(|r| (r - 5.0).abs() < 1e-6), "the pixel is not on the wall of the hole: {hit}");
    let body = json!({ "body": hit["body"] });
    let read = ok(call(&mut ctx, "fit_at_point", json!({ "body": body, "at": hit["at"] })));
    assert_eq!(read["found"]["kind"], "hole", "{read}");
    assert_eq!(read["found"]["size"], json!(10.0), "{read}");
    let made = ok(call(&mut ctx, "recognise_feature", json!({ "body": body, "at": hit["at"] })));
    assert_eq!(made["found"]["kind"], "hole", "{made}");
}

/// BY EYE ON A MESH: the drilled block written as an STL and brought back, made a polyhedron by `recognise_mesh` - its
/// hole a ring of flat strips, no round face anywhere - is read and made editable the same way: a strip of the wall
/// under the pixel, a hole of 10.
#[test]
fn a_hole_seen_on_a_mesh_is_made_editable() {
    let Drilled { mut ctx, .. } = drilled();
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-fit-at-point"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("drilled.stl").to_string_lossy().into_owned();
    let _ = ok(call(&mut ctx, "export_mesh", json!({ "path": stl })));
    let _ = ok(call(&mut ctx, "new_project", json!({})));
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl })));
    // a mesh is triangles, not a body: made a polyhedron of its flat faces first, as the window's Recognise "as is"
    let _ = ok(call(&mut ctx, "recognise_mesh", json!({ "body": { "body": mesh["bodies"][0]["id"] }, "mode": "as_is" })));
    let look = Look { view: View::Iso, size: Size { width: 800, height: 600 }, lit: Lit::Nothing, edges: Edges::Drawn, window: None };
    let r = 5.0 / std::f64::consts::SQRT_2;
    let px = picture::pixel_of(ctx.doc.project(), &look, [-r, r, 8.0]).expect("the wall is in the picture");
    let hit = ok(call(&mut ctx, "point_at", json!({ "view": "iso", "x": px.x, "y": px.y })));
    let body = json!({ "body": hit["body"] });
    let read = ok(call(&mut ctx, "fit_at_point", json!({ "body": body, "at": hit["at"] })));
    assert_eq!(read["found"]["kind"], "hole", "{read} under {hit}");
    assert_eq!(read["found"]["size"], json!(10.0), "{read}");
    let made = ok(call(&mut ctx, "recognise_feature", json!({ "body": body, "at": hit["at"] })));
    assert_eq!(made["found"]["kind"], "hole", "{made}");
}

/// A HOLE WHOSE RIM IS ROUNDED reads through, and widened it keeps its rim. Reported behaviour: the hole of the plate
/// whose top edges were all rounded 1.5 read blind 6.5 deep from the bottom, and recognise_feature laid a red node.
#[test]
fn a_hole_with_a_rounded_rim_reads_through_and_widens() {
    let Drilled { mut ctx, body } = drilled();
    let rounded = ok(call(&mut ctx, "fillet", json!({ "body": body, "edges": { "adjacent": "top" }, "radius": 1.5 })));
    let body = json!({ "body": rounded["body"] });
    let read = ok(call(&mut ctx, "fit_at_point", json!({ "body": body, "at": [0.0, 5.0, 4.0] })));
    assert_eq!(read["found"]["through"], json!(true), "{read}");
    let before = rounded["bodies"][0]["volume"].as_f64().expect("a volume");
    let made = ok(call(&mut ctx, "recognise_feature", json!({ "body": body, "at": [0.0, 5.0, 4.0], "size": 12 })));
    assert_eq!(made["red"], json!([]), "{made}");
    let after = made["bodies"][0]["volume"].as_f64().expect("a volume");
    // 12 across instead of 10, through the 10 of the block: the rim of 1.5 stands round the wider mouth as before
    let wider = std::f64::consts::PI * (36.0 - 25.0) * 10.0;
    assert!((before - after - wider).abs() < 5.0, "widened by {} mm^3, not about {wider}: {made}", before - after);
}

/// THE PLATE WITH ROUNDED CORNERS WRITTEN AS AN STL gives its holes and roundings and no chamfer. Reported behaviour:
/// recognise_all listed two chamfers, 43.1 and 30.7, on the plate of the manual check.
#[test]
fn a_plate_through_an_stl_lists_no_chamfer() {
    let mut ctx = Ctx::blank();
    let plate = ok(call(&mut ctx, "box", json!({ "x": 60, "y": 40, "z": 8 })));
    let top = ok(call(&mut ctx, "create_sketch", json!({ "plane": { "body": { "body": plate["body"] }, "face": "top" } })));
    let _ = ok(call(&mut ctx, "sketch_add", json!({ "sketch": top["sketch"], "entities": [{ "point": { "at": [-18, 0] } }, { "point": { "at": [18, 0] } }] })));
    let _ = ok(call(&mut ctx, "hole", json!({ "sketch": top["sketch"], "diameter": 6, "depth": 20 })));
    let side = |axis: &str, max: bool| json!({ "extreme": { "axis": axis, "max": max } });
    let corner = |x: bool, y: bool| json!({ "between": { "one": side("x", x), "other": side("y", y) } });
    let _ = ok(call(&mut ctx, "fillet", json!({ "edges": { "union": [corner(false, false), corner(false, true), corner(true, false), corner(true, true)] }, "radius": 5 })));
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-fit-at-point"));
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("rounded-plate.stl").to_string_lossy().into_owned();
    let _ = ok(call(&mut ctx, "export_mesh", json!({ "path": stl, "overwrite": true })));
    let _ = ok(call(&mut ctx, "new_project", json!({})));
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl })));
    let body = ok(call(&mut ctx, "recognise_mesh", json!({ "body": { "body": mesh["bodies"][0]["id"] } })));
    let listed = ok(call(&mut ctx, "recognise_all", json!({ "body": { "body": body["body"] }, "then": "list" })));
    let kinds: Vec<&str> = listed["groups"].as_array().expect("groups").iter().filter_map(|g| g["kind"].as_str()).collect();
    assert_eq!(kinds, ["hole", "rounding"], "{listed}");
}
