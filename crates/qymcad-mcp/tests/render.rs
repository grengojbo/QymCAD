//! THE DOCUMENT AS A PICTURE through the server: the PNG goes to the client as an image of its own, not as text; the
//! camera stands on the side asked for and frames the model by the window's fit, so a 60 x 40 x 4 plate is a 3:2
//! rectangle from above and a thin band from the front; a lit body is drawn lit; `render: true` on a change hands
//! the picture back with it; nothing to draw, or a size out of bounds, is refused.

use qymcad_mcp::picture::{self, Edges, Lit, Look, Size, View};
use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

/// The whole reply of a call, as the client reads it.
fn reply(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"].clone()
}

fn call(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    reply(ctx, name, arguments)["structuredContent"].clone()
}

fn ok(reply: Value) -> Value {
    assert_eq!(reply["ok"], json!(true), "{reply}");
    reply
}

/// The plate 60 x 40 x 4 from the origin.
fn plate() -> Ctx {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 60, "y": 40, "z": 4 })));
    ctx
}

/// The image a reply carries, and its size read from the PNG's own header.
struct Carried {
    width: u32,
    height: u32,
}

fn carried(reply: &Value) -> Option<Carried> {
    let content = reply["content"].as_array()?;
    let image = content.iter().find(|c| c["type"] == "image")?;
    assert_eq!(image["mimeType"], "image/png", "{image}");
    let data = decode(image["data"].as_str()?);
    // the signature, then IHDR: width and height big-endian at bytes 16..24
    assert_eq!(&data[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
    let be = |at: usize| u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
    Some(Carried { width: be(16), height: be(20) })
}

/// Base64 back to bytes, written apart from the server's encoder so that a fault in one is not hidden by the other.
fn decode(text: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => panic!("{c} is not base64"),
    };
    let mut out = Vec::new();
    for quad in text.as_bytes().chunks(4) {
        let digits: Vec<u8> = quad.iter().copied().filter(|c| *c != b'=').map(value).collect();
        let n = digits.iter().enumerate().fold(0u32, |n, (i, d)| n | u32::from(*d) << (18 - 6 * i));
        for k in 0..digits.len().saturating_sub(1) {
            out.push((n >> (16 - 8 * k)) as u8);
        }
    }
    out
}

/// The box the model fills in a picture: everything that is not the ground of its corner.
struct Filled {
    width: usize,
    height: usize,
}

fn filled(img: &egui::ColorImage) -> Filled {
    let ground = img.pixels[0];
    let [w, h] = img.size;
    let mut lo = [usize::MAX; 2];
    let mut hi = [0usize; 2];
    for y in 0..h {
        for x in 0..w {
            if img.pixels[y * w + x] != ground {
                lo = [lo[0].min(x), lo[1].min(y)];
                hi = [hi[0].max(x), hi[1].max(y)];
            }
        }
    }
    assert!(lo[0] <= hi[0], "nothing is drawn");
    Filled { width: hi[0] - lo[0] + 1, height: hi[1] - lo[1] + 1 }
}

fn look(view: View, lit: Lit) -> Look {
    Look { view, size: Size { width: 600, height: 600 }, lit, edges: Edges::Left }
}

#[test]
fn a_picture_goes_to_the_client_as_an_image() {
    let mut ctx = plate();
    let r = reply(&mut ctx, "render", json!({ "view": "iso", "width": 320, "height": 200 }));
    let shown = carried(&r).unwrap_or_else(|| panic!("no image in {r}"));
    assert_eq!([shown.width, shown.height], [320, 200], "the picture is not the size asked for");
    let body = &r["structuredContent"];
    assert_eq!(body["ok"], json!(true), "{body}");
    assert!(body.get("png").is_none(), "the picture stays in the JSON as text too");
    assert_eq!(body["view"], "iso", "{body}");
}

/// THE CAMERA ON THE SIDE ASKED FOR, the model filling 0.9 of the picture along the side it bounds first. In 600 x
/// 600 the plate is 540 x 360 from above (9 px per mm), 540 x 36 from the front (9 px per mm), 540 x 54 from the left
/// (13.5 px per mm).
#[test]
fn each_view_shows_the_plate_from_its_side() {
    let ctx = plate();
    struct Seen {
        view: View,
        width: usize,
        height: usize,
    }
    let wanted = [Seen { view: View::Top, width: 540, height: 360 }, Seen { view: View::Front, width: 540, height: 36 }, Seen { view: View::Left, width: 540, height: 54 }];
    let mut wrong = Vec::new();
    for s in wanted {
        let img = picture::draw(ctx.doc.project(), &look(s.view, Lit::Nothing), &[]).expect("drawn");
        let f = filled(&img);
        if f.width.abs_diff(s.width) > 2 || f.height.abs_diff(s.height) > 2 {
            wrong.push(format!("{:?}: {} x {}, not {} x {}", s.view, f.width, f.height, s.width, s.height));
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}

#[test]
fn a_lit_body_is_drawn_lit() {
    let ctx = plate();
    let project = ctx.doc.project();
    let body = project.bodies[0].id;
    let plain = picture::draw(project, &look(View::Iso, Lit::Nothing), &[]).expect("drawn");
    let lit = picture::draw(project, &look(View::Iso, Lit::Body(body)), &[]).expect("drawn");
    let changed = plain.pixels.iter().zip(&lit.pixels).filter(|(a, b)| a != b).count();
    let model = plain.pixels.iter().filter(|p| **p != plain.pixels[0]).count();
    assert!(changed * 2 > model, "lighting the body changed {changed} pixels of the {model} it covers");
}

#[test]
fn a_change_asked_to_draw_itself_hands_the_picture_back() {
    let mut ctx = Ctx::blank();
    let r = reply(&mut ctx, "box", json!({ "x": 20, "y": 20, "z": 20, "render": true }));
    assert_eq!(r["structuredContent"]["ok"], json!(true), "{r}");
    let shown = carried(&r).unwrap_or_else(|| panic!("no image in {r}"));
    assert_eq!([shown.width, shown.height], [800, 600]);
    assert_eq!(r["structuredContent"]["picture"]["view"], "iso", "{r}");
    // without it, no picture
    let r = reply(&mut ctx, "box", json!({ "x": 5, "y": 5, "z": 5 }));
    assert!(carried(&r).is_none(), "a picture nobody asked for: {r}");
    // every tool but render lists it
    let listed = answer(&mut ctx, &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }).to_string()).expect("listed");
    let tools = listed["result"]["tools"].as_array().expect("tools");
    let without: Vec<&str> = tools.iter().filter(|t| t["name"] != "render" && t["inputSchema"]["properties"].get("render").is_none()).filter_map(|t| t["name"].as_str()).collect();
    assert!(without.is_empty(), "tools without render: {without:?}");
}

#[test]
fn nothing_to_draw_and_a_size_out_of_bounds_are_refused() {
    let mut ctx = Ctx::blank();
    let r = call(&mut ctx, "render", json!({}));
    assert_eq!(r["error"]["code"], "no-body", "{r}");
    let mut ctx = plate();
    for size in [json!({ "width": 10 }), json!({ "height": 5000 })] {
        let r = call(&mut ctx, "render", size.clone());
        assert_eq!(r["error"]["code"], "bad-size", "{size}: {r}");
    }
    let r = call(&mut ctx, "render", json!({ "highlight": { "body": 999 } }));
    assert_eq!(r["error"]["code"], "no-body", "{r}");
}

/// THE EDGES BEHIND THE BODY ARE NOT DRAWN. A cube 20 from iso shows nine of its twelve edges: the six of its outline
/// and the three of the near corner; the three of the far corner run behind it. The cube stands 2 L high on the screen
/// and is framed to 0.9 x 600 = 540 px by that height, so an edge spans L = 270 px; a line is drawn as many pixels as
/// its longer screen side: L for an upright edge, L cos 30 deg for the others. Nine edges - three upright, six slanted - are 8.2 L of pixels; the three hidden
/// ones would add 2.7 L.
#[test]
fn the_edges_behind_the_body_are_not_drawn() {
    let mut ctx = Ctx::blank();
    let _ = ok(call(&mut ctx, "box", json!({ "x": 20, "y": 20, "z": 20 })));
    let mut seen = look(View::Iso, Lit::Nothing);
    seen.edges = Edges::Drawn;
    let lines = picture::lines(&ctx.doc);
    assert_eq!(lines.len(), 12, "a cube has 12 edges");
    let img = picture::draw(ctx.doc.project(), &seen, &lines).expect("drawn");
    // an edge is drawn in the light scheme's colour of edges, which neither the bodies nor the ground take
    let pen = qymcad_scheme::light().edge_idle();
    let drawn = img.pixels.iter().filter(|p| **p == pen).count() as f64;
    let l = 270.0;
    let edges = drawn / l;
    assert!((7.6..=8.8).contains(&edges), "the picture draws {edges:.2} L of edges, not the 8.2 L of the nine in sight");
}
