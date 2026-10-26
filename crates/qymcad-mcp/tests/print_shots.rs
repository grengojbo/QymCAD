//! THE PICTURES OF THE MANUAL CHECK OF GETTING READY TO PRINT, drawn by the server itself: a can from an STL with holes
//! before and after it is mended and made lighter, a plate stood on its end with the places that hang marked and the same
//! plate laid the best way, a bar too long for the printer cut in two with its pegs, the pieces drawn apart - and the
//! answers of each call printed beside the name of the picture, so the text of the check quotes the server.
//!
//! To draw: `cargo test -p qymcad-mcp --test print_shots -- --ignored --nocapture`. The pictures go to
//! `target/tutorial/31-print/`.

use qymcad_mcp::picture::{self, Edges, Lit, Look, Size, Triangles, View};
use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    let r = answer(ctx, &line).expect("a request is answered")["result"]["structuredContent"].clone();
    assert_eq!(r["ok"], json!(true), "{name}: {r}");
    r
}

/// The pictures of the check, numbered in the order they are drawn.
struct Shots {
    dir: std::path::PathBuf,
    n: usize,
}

/// How a picture is drawn: the triangles of a mesh drawn over it, or left out.
struct Shot<'a> {
    name: &'a str,
    triangles: Triangles,
    marks: &'a [[f64; 3]],
}

impl Shots {
    /// The picture from iso at 800 x 600, written as `NN-name.png`.
    fn take(&mut self, ctx: &Ctx, shot: Shot) {
        let look = Look { view: View::Iso, size: Size { width: 800, height: 600 }, lit: Lit::Nothing, edges: Edges::Drawn, triangles: shot.triangles, window: None };
        let drawn = picture::png_marked(&ctx.doc, &look, shot.marks).expect("drawn");
        let file = format!("{:02}-{}.png", self.n, shot.name);
        self.n += 1;
        std::fs::write(self.dir.join(&file), drawn.png).expect("writing the picture");
        println!("SHOT {file}");
    }
}

/// A closed can of radius 10 and height 20 round z, `around` sides, its ends fans to their middles, as ASCII STL; the
/// side triangles `left_out` are not written.
fn can_stl(around: u32, left_out: &[u32]) -> String {
    let at = |k: u32, z: f64| {
        let a = std::f64::consts::TAU * f64::from(k % around) / f64::from(around);
        [10.0 * a.cos(), 10.0 * a.sin(), z]
    };
    let mut out = String::from("solid can\n");
    for k in 0..around {
        let tris = [
            [at(k, 0.0), at(k + 1, 0.0), at(k + 1, 20.0)],
            [at(k, 0.0), at(k + 1, 20.0), at(k, 20.0)],
            [[0.0, 0.0, 0.0], at(k + 1, 0.0), at(k, 0.0)],
            [[0.0, 0.0, 20.0], at(k, 20.0), at(k + 1, 20.0)],
        ];
        for (j, t) in tris.iter().enumerate() {
            if j < 2 && left_out.contains(&(2 * k + j as u32)) {
                continue;
            }
            out.push_str("facet normal 0 0 0\nouter loop\n");
            for p in t {
                out.push_str(&format!("vertex {} {} {}\n", p[0], p[1], p[2]));
            }
            out.push_str("endloop\nendfacet\n");
        }
    }
    out + "endsolid can\n"
}

/// The places a check of printing names as hanging.
fn hanging(check: &Value) -> Vec<[f64; 3]> {
    let places = check["checks"]["overhang"]["places"].as_array().cloned().unwrap_or_default();
    places.iter().filter_map(|p| p["at"].as_array().map(|a| a.iter().filter_map(Value::as_f64).collect::<Vec<f64>>())).filter(|a| a.len() == 3).map(|a| [a[0], a[1], a[2]]).collect()
}

#[test]
#[ignore = "draws the pictures of the manual check; run by hand"]
fn print_shots() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/tutorial/31-print"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder of the pictures");
    let mut shots = Shots { dir: dir.clone(), n: 0 };

    // US-4: a can of 120 sides with six side triangles missing on the side the iso eye looks at (-45 deg, k = 105)
    let stl = dir.join("can.stl");
    std::fs::write(&stl, can_stl(120, &[208, 209, 210, 211, 212, 213])).expect("the can written");
    let mut ctx = Ctx::blank();
    let mesh = call(&mut ctx, "import_mesh", json!({ "path": stl.to_string_lossy() }));
    let body = json!({ "body": mesh["bodies"][0]["id"] });
    println!("mesh_report before: {}", call(&mut ctx, "mesh_report", json!({ "body": body })));
    shots.take(&ctx, Shot { name: "can-with-holes", triangles: Triangles::Drawn, marks: &[] });
    let mended = call(&mut ctx, "mesh_repair", json!({ "body": body, "fill_holes": 64, "simplify": 0.1 }));
    println!("mesh_repair: closed_holes {} before {} after {}", mended["closed_holes"], mended["before"]["triangles"], mended["after"]["triangles"]);
    println!("mesh_report after: {}", call(&mut ctx, "mesh_report", json!({ "body": body })));
    println!("check_printability: {}", call(&mut ctx, "check_printability", json!({ "body": body, "printer": "p1s" }))["verdict"]);
    shots.take(&ctx, Shot { name: "can-mended", triangles: Triangles::Drawn, marks: &[] });

    // US-6: a plate 60 x 40 x 5 with a countersunk hole, stood on its end
    let mut ctx = Ctx::blank();
    let _ = call(&mut ctx, "box", json!({ "x": 60, "y": 40, "z": 5 }));
    let _ = call(&mut ctx, "hole", json!({ "face": "top", "diameter": 3.4, "depth": 20, "kind": "countersink", "recess_diameter": 6.5, "recess_depth": 1.6 }));
    let _ = call(&mut ctx, "place_on_face", json!({ "face": { "extreme": { "axis": "x", "max": true } } }));
    let standing = call(&mut ctx, "check_printability", json!({ "printer": "p1s" }));
    println!("check_printability standing: {standing}");
    shots.take(&ctx, Shot { name: "plate-standing", triangles: Triangles::Left, marks: &hanging(&standing) });
    let ways = call(&mut ctx, "suggest_orientation", json!({}));
    println!("suggest_orientation: {ways}");
    let laid = call(&mut ctx, "place_on_face", ways["ways"][0]["lay"].clone());
    println!("place_on_face: size {} lowest {}", laid["size"], laid["lowest"]);
    let lying = call(&mut ctx, "check_printability", json!({ "printer": "p1s" }));
    println!("check_printability laid: {}", lying["verdict"]);
    shots.take(&ctx, Shot { name: "plate-laid", triangles: Triangles::Left, marks: &[] });

    // US-7: a bar of 300 x 20 x 20 for a room of 256, cut with pegs; the pieces drawn 40 apart to show them
    let mut ctx = Ctx::blank();
    let bar = call(&mut ctx, "box", json!({ "x": 300, "y": 20, "z": 20 }));
    println!("check_printability whole: {}", call(&mut ctx, "check_printability", json!({ "printer": "p1s" }))["checks"]["bed"]);
    let cut = call(&mut ctx, "split_for_bed", json!({ "body": { "body": bar["body"] }, "printer": "p1s", "joints": {} }));
    println!("split_for_bed: pieces {} joints {}", cut["pieces"], cut["joints"]);
    shots.take(&ctx, Shot { name: "bar-cut", triangles: Triangles::Left, marks: &[] });
    let pieces = cut["pieces"].as_array().cloned().unwrap_or_default();
    for p in &pieces {
        let check = call(&mut ctx, "check_printability", json!({ "body": { "body": p["body"] }, "printer": "p1s" }));
        println!("check_printability piece {}: bed {} verdict {} overhang {}", p["body"], check["checks"]["bed"]["verdict"], check["verdict"], check["checks"]["overhang"]);
        let ways = call(&mut ctx, "suggest_orientation", json!({ "body": { "body": p["body"] } }));
        println!("suggest_orientation piece {}: {}", p["body"], ways["ways"][0]);
    }
    // the piece standing further along x is the one moved further along it
    let bodies = call(&mut ctx, "get_document", json!({}))["document"]["bodies"].as_array().cloned().unwrap_or_default();
    let low = |b: &Value| bodies.iter().find(|d| d["id"] == b["body"]).and_then(|d| d["min"][0].as_f64()).unwrap_or(f64::MIN);
    let far = pieces.iter().max_by(|a, b| low(a).total_cmp(&low(b))).cloned().unwrap_or_default();
    let _ = call(&mut ctx, "move", json!({ "body": { "body": far["body"] }, "at": [40, 0, 0] }));
    shots.take(&ctx, Shot { name: "bar-apart", triangles: Triangles::Left, marks: &[] });
}
