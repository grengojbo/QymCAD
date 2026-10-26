//! FROM A PIXEL OF A PICTURE TO A PLACE ON THE MODEL: `point_at` takes the side and the size `render` drew from and a
//! pixel of that picture, and answers what the window's pointer would catch there - the body, the face, the point in
//! the world and in the body's own frame, the one `recognise_feature` and `fit_at_point` take. A place of the model
//! carried to its pixel and back comes out where it was; the ground answers a refusal, not the nearest body.

use qymcad_mcp::picture::{Triangles, self, Edges, Lit, Look, Size, View};
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

/// A block 60 x 40 x 10 standing on the origin, centred on it in X and Y.
fn block() -> Ctx {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 60, "y": 40, "z": 10 })));
    ctx
}

fn look(view: View) -> Look {
    Look { view, size: Size { width: 800, height: 600 }, lit: Lit::Nothing, edges: Edges::Drawn, triangles: Triangles::Left, window: None }
}

fn point(v: &Value) -> [f64; 3] {
    let n = |k: usize| v[k].as_f64().unwrap_or_else(|| panic!("no number {k} in {v}"));
    [n(0), n(1), n(2)]
}

/// A place, measured from.
struct Place {
    at: [f64; 3],
}

fn at(p: [f64; 3]) -> Place {
    Place { at: p }
}

impl Place {
    fn to(&self, b: [f64; 3]) -> f64 {
        let a = self.at;
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }
}

/// A place on each of the three faces the iso view shows, carried to its pixel and asked back: the same face, the
/// same place within a pixel - 0.2 mm at 800 x 600 for a block 60 long.
#[test]
fn a_place_carried_to_its_pixel_comes_back() {
    let mut ctx = block();
    struct Place {
        world: [f64; 3],
        normal: [f64; 3],
    }
    let places = [
        Place { world: [5.0, 5.0, 10.0], normal: [0.0, 0.0, 1.0] },
        Place { world: [0.0, -20.0, 5.0], normal: [0.0, -1.0, 0.0] },
        Place { world: [30.0, 5.0, 4.0], normal: [1.0, 0.0, 0.0] },
    ];
    let mut wrong = Vec::new();
    for p in places {
        let px = picture::pixel_of(ctx.doc.project(), &look(View::Iso), p.world).expect("the place is in the picture");
        let r = call(&mut ctx, "point_at", json!({ "view": "iso", "width": 800, "height": 600, "x": px.x, "y": px.y }));
        if r["ok"] != json!(true) {
            wrong.push(format!("{:?}: {r}", p.world));
            continue;
        }
        let world = point(&r["world"]);
        if at(world).to(p.world) > 0.2 {
            wrong.push(format!("{:?} came back as {world:?}", p.world));
        }
        if at(point(&r["face"]["normal"])).to(p.normal) > 1e-6 {
            wrong.push(format!("{:?} at pixel {px:?}: the face facing {} is not the one facing {:?}: {r}", p.world, r["face"]["normal"], p.normal));
        }
        if at(point(&r["at"])).to(world) > 1e-6 {
            wrong.push(format!("{:?}: a body standing where it was made is in its own frame where it is in the world: {} and {world:?}", p.world, r["at"]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The answer names the body and its face as the other tools name them, and the face as `list_faces` reads it.
#[test]
fn the_face_is_named_as_list_faces_names_it() {
    let mut ctx = block();
    let r = ok(call(&mut ctx, "point_at", json!({ "view": "top", "x": 400, "y": 300 })));
    let faces = ok(call(&mut ctx, "list_faces", json!({ "body": { "body": r["body"] } })));
    let listed = faces["faces"].as_array().expect("faces").iter().find(|f| f["key"] == r["face"]["key"]).cloned();
    assert_eq!(listed.as_ref(), Some(&r["face"]), "the face answered is not the one list_faces gives: {r}");
    assert_eq!(r["face"]["kind"], "plane", "{r}");
}

/// The ground of the picture holds nothing: refused, not the nearest body.
#[test]
fn the_ground_is_refused() {
    let mut ctx = block();
    let r = call(&mut ctx, "point_at", json!({ "view": "iso", "x": 2, "y": 2 }));
    assert_eq!(r["ok"], json!(false), "{r}");
    assert_eq!(r["error"]["code"], "nothing-there", "{r}");
}

/// A pixel off the picture, and a picture of a size `render` would refuse, are refused.
#[test]
fn a_pixel_off_the_picture_is_refused() {
    let mut ctx = block();
    let off = call(&mut ctx, "point_at", json!({ "view": "iso", "width": 320, "height": 200, "x": 400, "y": 100 }));
    assert_eq!(off["error"]["code"], "off-picture", "{off}");
    let small = call(&mut ctx, "point_at", json!({ "view": "iso", "width": 10, "height": 200, "x": 5, "y": 100 }));
    assert_eq!(small["error"]["code"], "bad-size", "{small}");
}
