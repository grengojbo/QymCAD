//! THE PICTURES OF THE MANUAL CHECK OF RECOGNISING BY EYE: the pictures Claude is given on the way, drawn by the server
//! itself - a plate with two holes and rounded corners written as an STL, brought back, made a body, looked at, aimed
//! at with marks, a hole made editable - and the answers of each call printed beside the name of the picture, so the
//! text of the check quotes the server rather than a memory of it.
//!
//! To draw: `cargo test -p qymcad-mcp --test by_eye_shots -- --ignored --nocapture`. The pictures go to
//! `target/tutorial/29-by-eye/`.

use qymcad_mcp::picture::{self, Edges, Lit, Look, Size, View};
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

impl Shots {
    /// The picture from iso at 800 x 600 with `marks` on it, written as `NN-name.png`.
    fn take(&mut self, ctx: &Ctx, name: &str, marks: &[[f64; 3]]) {
        let look = Look { view: View::Iso, size: Size { width: 800, height: 600 }, lit: Lit::Nothing, edges: Edges::Drawn, window: None };
        let shot = picture::png_marked(&ctx.doc, &look, marks).expect("drawn");
        let file = format!("{:02}-{name}.png", self.n);
        self.n += 1;
        std::fs::write(self.dir.join(&file), shot.png).expect("writing the picture");
        println!("SHOT {file}");
    }
}

/// A place on the wall of a hole of radius `r` about the vertical axis through `centre`, at its height, on the side the
/// iso eye looks into: the eye stands at +X -Y +Z, the wall it sees is the one at -X +Y.
fn far_wall(centre: [f64; 3], r: f64) -> [f64; 3] {
    let s = r / std::f64::consts::SQRT_2;
    [centre[0] - s, centre[1] + s, centre[2]]
}

#[test]
#[ignore = "draws the pictures of the manual check; run by hand"]
fn by_eye_shots() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/tutorial/29-by-eye"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder of the pictures");
    let mut shots = Shots { dir: dir.clone(), n: 0 };

    // the plate as a file from elsewhere: two holes of 6 through, written as an STL
    let mut ctx = Ctx::blank();
    let plate = call(&mut ctx, "box", json!({ "x": 60, "y": 40, "z": 8 }));
    let top = call(&mut ctx, "create_sketch", json!({ "plane": { "body": { "body": plate["body"] }, "face": "top" } }));
    let _ = call(&mut ctx, "sketch_add", json!({ "sketch": top["sketch"], "entities": [{ "point": { "at": [-18, 0] } }, { "point": { "at": [18, 0] } }] }));
    let _ = call(&mut ctx, "hole", json!({ "sketch": top["sketch"], "diameter": 6, "depth": 20 }));
    // the four upright corners rounded 5: a rounding a person sees, the rims of the holes left sharp
    let side = |axis: &str, max: bool| json!({ "extreme": { "axis": axis, "max": max } });
    for x in [false, true] {
        for y in [false, true] {
            let _ = call(&mut ctx, "fillet", json!({ "edges": { "between": { "one": side("x", x), "other": side("y", y) } }, "radius": 5 }));
        }
    }
    let stl = dir.join("plate.stl").to_string_lossy().into_owned();
    let _ = call(&mut ctx, "export_mesh", json!({ "path": stl }));

    let _ = call(&mut ctx, "new_project", json!({}));
    let mesh = call(&mut ctx, "import_mesh", json!({ "path": stl }));
    println!("import_mesh: {}", mesh["bodies"]);
    shots.take(&ctx, "mesh", &[]);

    let body = call(&mut ctx, "recognise_mesh", json!({ "body": { "body": mesh["bodies"][0]["id"] } }));
    let key = json!({ "body": body["body"] });
    println!("recognise_mesh: body {} {}", body["body"], body["bodies"]);
    shots.take(&ctx, "body", &[]);

    let listed = call(&mut ctx, "recognise_all", json!({ "body": key, "then": "list" }));
    println!("recognise_all list: {}", listed["groups"]);

    // the walls of the two holes, as Claude finds them on the picture: a pixel each, read back
    let look = Look { view: View::Iso, size: Size { width: 800, height: 600 }, lit: Lit::Nothing, edges: Edges::Drawn, window: None };
    let mut aims = Vec::new();
    for centre in [[-18.0, 0.0, 5.0], [18.0, 0.0, 5.0]] {
        let px = picture::pixel_of(ctx.doc.project(), &look, far_wall(centre, 3.0)).expect("the wall is in the picture");
        let hit = call(&mut ctx, "point_at", json!({ "view": "iso", "x": px.x.round(), "y": px.y.round() }));
        println!("point_at {} {}: face {} at {}", px.x.round(), px.y.round(), hit["face"]["kind"], hit["at"]);
        let read = call(&mut ctx, "fit_at_point", json!({ "body": key, "at": hit["at"] }));
        println!("fit_at_point: {}", read["found"]);
        let at = hit["world"].as_array().expect("a place").iter().filter_map(Value::as_f64).collect::<Vec<f64>>();
        aims.push([at[0], at[1], at[2]]);
    }
    shots.take(&ctx, "aimed", &aims);

    let made = call(&mut ctx, "recognise_feature", json!({ "body": key, "at": aims[0], "size": 8 }));
    println!("recognise_feature size 8: {} {}", made["found"], made["bodies"]);
    shots.take(&ctx, "hole-8", &[]);
}
