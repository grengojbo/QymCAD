//! THE HEALTH OF A MESH THROUGH THE SERVER: a box of faces reads closed, whole and of its volume; a cube from STL short
//! of one triangle reads open along one hole of three edges, the hole a slicer would meet.

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

/// A cube of side 10 from the origin as a text STL, less the triangles numbered in `left_out`.
fn cube_stl(left_out: &[usize]) -> String {
    let c = |i: usize| [(i & 1) as f64 * 10.0, ((i >> 1) & 1) as f64 * 10.0, ((i >> 2) & 1) as f64 * 10.0];
    let tris = [[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    let mut out = String::from("solid cube\n");
    for (_, t) in tris.iter().enumerate().filter(|(k, _)| !left_out.contains(k)) {
        out.push_str("facet normal 0 0 0\nouter loop\n");
        for &i in t {
            let p = c(i);
            out.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
        }
        out.push_str("endloop\nendfacet\n");
    }
    out + "endsolid cube\n"
}

#[test]
fn a_box_of_faces_reads_closed() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 10 })));
    let r = ok(call(&mut ctx, "mesh_report", json!({})));
    assert_eq!(r["closed"], json!(true), "{r}");
    assert_eq!(r["holes"], json!(0), "{r}");
    assert_eq!(r["pieces"], json!(1), "{r}");
    assert!((r["volume"].as_f64().unwrap_or(0.0) - 1000.0).abs() < 1e-3, "{r}");
}

#[test]
fn a_cube_from_stl_short_of_a_triangle_reads_one_hole() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-mcp-mesh-report"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("open.stl");
    std::fs::write(&stl, cube_stl(&[2])).expect("the cube written");
    let mut ctx = Ctx::blank();
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl.to_string_lossy() })));
    let r = ok(call(&mut ctx, "mesh_report", json!({ "body": { "body": mesh["bodies"][0]["id"] } })));
    assert_eq!(r["triangles"], json!(11), "{r}");
    assert_eq!(r["holes"], json!(1), "{r}");
    assert_eq!(r["open_edges"], json!(3), "{r}");
    assert_eq!(r["closed"], json!(false), "{r}");
}

/// A server with a mesh brought in, and the body's key.
struct Brought {
    ctx: Ctx,
    body: Value,
}

/// The cube from STL short of a triangle, brought in.
fn open_cube(check: &str) -> Brought {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target")).join(format!("qymcad-mcp-mesh-{check}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("open.stl");
    std::fs::write(&stl, cube_stl(&[2])).expect("the cube written");
    let mut ctx = Ctx::blank();
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl.to_string_lossy() })));
    let body = json!({ "body": mesh["bodies"][0]["id"] });
    Brought { ctx, body }
}

/// MENDED AS ONE STEP: the hole of a cube from STL is closed, mesh_report reads the cube whole, and undo brings the
/// hole back.
#[test]
fn a_hole_mended_is_closed_and_undo_opens_it_again() {
    let Brought { mut ctx, body } = open_cube("repair");
    let r = ok(call(&mut ctx, "mesh_repair", json!({ "body": body })));
    assert_eq!(r["closed_holes"], json!(1), "{r}");
    assert_eq!(r["before"]["holes"], json!(1), "{r}");
    assert_eq!(r["after"]["closed"], json!(true), "{r}");
    let read = ok(call(&mut ctx, "mesh_report", json!({ "body": body })));
    assert_eq!(read["closed"], json!(true), "the document does not hold the mended mesh: {read}");
    assert!((read["volume"].as_f64().unwrap_or(0.0) - 1000.0).abs() < 1e-3, "{read}");
    let _ = ok(call(&mut ctx, "undo", json!({})));
    let back = ok(call(&mut ctx, "mesh_report", json!({ "body": body })));
    assert_eq!(back["holes"], json!(1), "undo did not bring the hole back: {back}");
}

/// A BODY OF FACES, OR A MESH ALREADY USED, IS NOT MENDED: refused with its code, the document as it was.
#[test]
fn a_body_of_faces_or_a_mesh_used_is_refused() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 10, "y": 10, "z": 10 })));
    let r = call(&mut ctx, "mesh_repair", json!({}));
    assert_eq!(r["error"]["code"], "not-a-mesh", "{r}");
    let Brought { mut ctx, body } = open_cube("used");
    let _ = ok(call(&mut ctx, "recognise_mesh", json!({ "body": body, "mode": "as_is" })));
    let r = call(&mut ctx, "mesh_repair", json!({ "body": body }));
    // the step that took it in stands in its place
    assert_eq!(r["error"]["code"], "consumed-body", "{r}");
}

/// A cube of 10 with a hollow cube of 2 in its middle as a text STL, the hollow's faces looking into the solid.
fn hollow_cube_stl() -> String {
    let corner = |side: f64, at: f64, i: usize| [at + (i & 1) as f64 * side, at + ((i >> 1) & 1) as f64 * side, at + ((i >> 2) & 1) as f64 * side];
    let tris = [[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    let mut out = String::from("solid hollow\n");
    for t in tris {
        let outer = t.map(|i| corner(10.0, 0.0, i));
        let inner = [t[0], t[2], t[1]].map(|i| corner(2.0, 4.0, i));
        for tri in [outer, inner] {
            out.push_str("facet normal 0 0 0\nouter loop\n");
            for p in tri {
                out.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
            }
            out.push_str("endloop\nendfacet\n");
        }
    }
    out + "endsolid hollow\n"
}

/// A HOLLOW LEFT ON PURPOSE is told as a hollow, not as triangles wound inward, and mesh_repair keeps it: the cube
/// closes 1 000 - 8 before and after.
#[test]
fn a_hollow_is_told_and_kept() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target")).join("qymcad-mcp-mesh-hollow");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let stl = dir.join("hollow.stl");
    std::fs::write(&stl, hollow_cube_stl()).expect("the cube written");
    let mut ctx = Ctx::blank();
    let mesh = ok(call(&mut ctx, "import_mesh", json!({ "path": stl.to_string_lossy() })));
    let body = json!({ "body": mesh["bodies"][0]["id"] });
    let r = ok(call(&mut ctx, "mesh_report", json!({ "body": body })));
    assert_eq!(r["flipped"], json!(0), "{r}");
    assert_eq!(r["hollows"], json!(1), "{r}");
    assert_eq!(r["pieces"], json!(1), "{r}");
    assert!((r["volume"].as_f64().unwrap_or(0.0) - 992.0).abs() < 1e-3, "{r}");
    let _ = ok(call(&mut ctx, "mesh_repair", json!({ "body": body, "drop_under": 10 })));
    let after = ok(call(&mut ctx, "mesh_report", json!({ "body": body })));
    assert_eq!(after["hollows"], json!(1), "mesh_repair took the hollow away: {after}");
    assert!((after["volume"].as_f64().unwrap_or(0.0) - 992.0).abs() < 1e-3, "{after}");
    let _ = std::fs::remove_dir_all(&dir);
}
