//! A DOCUMENT THE SERVER WROTE IS ONE THE WINDOW OPENS, and both give the same account of it.
//!
//! The server and the window measure a document through two doors - the server's account and the window's own - and
//! a model reading one while a person reads the other must see one document. The chain touches what the account
//! reads: a block with a pocket on its face and a rounded rim (a cut that consumes its source, a reference by a
//! query), a second part with a box, a parameter, and an STL brought in (a body with no exact faces). The window
//! opens the file the way a person does, File and Open, and its account is held against the server's.

use qymcad_core::feature::{Extent, FaceKey, Purpose, Reach, SketchPlane};
use qymcad_core::model::{CombineSpan, Id, Project};
use qymcad_core::refs::{Axis, Query, Ref};
use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    let reply = answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone();
    assert_eq!(reply["ok"], json!(true), "{name} did not go through: {reply}");
    reply
}

fn out_dir() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-gui"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

/// The corners of a rectangle in its sketch.
struct Corners {
    lo: [f64; 2],
    hi: [f64; 2],
}

/// A sketch and the closed contour in it.
struct Profile {
    sketch: Id,
    contour: Id,
}

/// A closed rectangle in a new sketch on `plane`.
fn rect(p: &mut Project, plane: Option<SketchPlane>, Corners { lo, hi }: Corners) -> Profile {
    let si = p.new_sketch("profile");
    if let Some(plane) = plane {
        p.sketches[si].plane = plane;
    }
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "profile");
    let _ = p.add_rect_entity(si, lo[0], lo[1], hi[0], hi[1], Purpose::Real);
    p.regen_sketch(si);
    let contour = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("a closed contour");
    Profile { sketch: sid, contour }
}

/// The server's document: the chain of the rebuild's fingerprint in the first part, a box in a second part, a
/// parameter, and an imported STL.
fn served(dir: &std::path::Path) -> Ctx {
    let mut ctx = Ctx::blank();
    let block = ctx
        .doc
        .edit("block", |p| {
            let Profile { sketch: sid, contour: cid } = rect(p, None, Corners { lo: [0.0, 0.0], hi: [40.0, 30.0] });
            let e = p.add_extrude_multi(sid, vec![cid], 10.0, Reach::Forward, 0.0, vec![]);
            Ok(p.finish_base_body(e, 1))
        })
        .expect("the block is laid");
    let cut = ctx
        .doc
        .edit("pocket", |p| {
            let top = p.regen_faces.get(&block).and_then(|fs| fs.iter().find(|f| f.normal[2] > 0.9)).cloned().ok_or("no top face")?;
            let key = FaceKey { index: 0, centroid: [top.centroid.x, top.centroid.y, top.centroid.z], normal: top.normal, id: top.id };
            let Profile { sketch: sid, contour: cid } = rect(p, Some(SketchPlane::Face(block, key)), Corners { lo: [10.0, 10.0], hi: [30.0, 20.0] });
            let span = CombineSpan { height: 4.0, down: 0.0, extent: Extent { reach: Reach::Backward, ..Default::default() }, fill: &[] };
            Ok(p.add_combine_multi_op(block, sid, vec![cid], span, 0))
        })
        .expect("the pocket is laid");
    let rim = Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })));
    let _ = ctx.doc.edit("fillet", |p| Ok(p.add_fillet_ref(cut, 1.0, rim))).expect("the fillet is laid");
    let _ = ctx
        .doc
        .edit("second part", |p| {
            p.active_component = Some(p.root);
            let part = p.add_part("Lid");
            p.active_component = Some(part);
            Ok(p.add_box(20.0, 10.0, 5.0))
        })
        .expect("the lid is laid");
    let _ = call(&mut ctx, "set_parameter", json!({ "name": "wall", "expr": "2 + 1" }));

    // an STL of a cube, written by another document and brought in: a body with no exact faces
    let stl = dir.join("cube.stl").to_string_lossy().into_owned();
    let mut other = Ctx::blank();
    let _ = other.doc.edit("cmd-box", |p| Ok(p.add_box(8.0, 8.0, 8.0))).expect("the cube is laid");
    let _ = call(&mut other, "export_mesh", json!({ "path": stl, "quality": "draft" }));
    let _ = call(&mut ctx, "import_mesh", json!({ "path": stl }));
    ctx
}

fn near(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(1.0)
}

fn near3(a: &Value, b: [f64; 3]) -> bool {
    (0..3).all(|i| a[i].as_f64().is_some_and(|x| near(x, b[i], 1e-6)))
}

#[test]
fn a_document_the_server_wrote_opens_in_the_window_the_same() {
    let dir = out_dir();
    let mut ctx = served(&dir);
    let path = dir.join("served.qcad").to_string_lossy().into_owned();
    let _ = call(&mut ctx, "save_project", json!({ "path": path }));
    let ours = call(&mut ctx, "get_document", json!({ "detail": "full" }))["document"].clone();

    let mut s = qymcad::Session::start();
    qymcad_acceptance::build::open_project(&mut s, &path);
    let theirs = s.document();
    assert_eq!(theirs.path.as_deref(), Some(path.as_str()), "the window did not open the file the server wrote");

    let parts = ours["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), theirs.parts.len(), "the parts: {parts:?} against {:?}", theirs.parts);
    for (o, t) in parts.iter().zip(&theirs.parts) {
        assert_eq!((o["key"].as_u64(), o["assembly"].as_bool(), o["visible"].as_bool()), (Some(t.key), Some(t.assembly), Some(t.visible)), "a part differs: {o} against {t:?}");
    }

    let features = ours["features"].as_array().expect("features");
    assert_eq!(features.len(), theirs.features.len(), "the timelines differ in length");
    for (o, t) in features.iter().zip(&theirs.features) {
        let same =
            o["key"].as_u64() == Some(t.key) && o["kind"] == json!(t.kind) && o["suppressed"] == json!(t.suppressed) && o["bodies"] == json!(t.bodies) && o["error"].is_null() == t.error.is_none();
        assert!(same, "a feature differs: {o} against {t:?}");
    }

    let bodies = ours["bodies"].as_array().expect("bodies");
    assert_eq!(bodies.len(), theirs.bodies.len(), "the bodies differ in number");
    assert!(bodies.iter().any(|b| b["edges"].is_null()), "the chain lost its mesh-only body: {bodies:?}");
    for (o, t) in bodies.iter().zip(&theirs.bodies) {
        let same = o["volume"].as_f64().is_some_and(|v| near(v, t.volume, 1e-6))
            && o["area"].as_f64().is_some_and(|a| near(a, t.area, 1e-6))
            && near3(&o["min"], t.min)
            && near3(&o["max"], t.max)
            && o["faces"] == json!(t.faces)
            && o["edges"] == json!(t.edges)
            && o["part"].as_u64() == t.part_key
            && o["consumed"] == json!(t.consumed)
            && o["visible"] == json!(t.visible)
            && o["sheet"] == json!(t.sheet);
        assert!(same, "a body differs:\n server {o}\n window {t:?}");
    }

    let params: Vec<Value> = theirs.parameters.iter().map(|p| json!({ "name": p.name, "expr": p.expr, "value": p.value })).collect();
    assert_eq!(ours["parameters"], json!(params), "the parameters differ");
}
