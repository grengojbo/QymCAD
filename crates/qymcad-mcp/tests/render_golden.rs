//! THE PICTURES HELD AGAINST THE ONES LOOKED AT: three parts, each from iso and from above, drawn through the server
//! and compared pixel by pixel with the pictures kept beside this file. Fewer than 1 % of the pixels may differ - the
//! room a tessellation of another machine takes at the rim of a hole; a camera turned, a fit changed, a body not drawn
//! or drawn in another colour takes far more.
//!
//! The kept pictures were looked at by eye. `QYM_BLESS=1` writes them anew from what is drawn now; after that they are
//! looked at again before they are kept.

use std::path::PathBuf;

use qymcad_mcp::server::answer;
use qymcad_mcp::tool::Ctx;
use serde_json::{json, Value};

fn reply(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name, "arguments": arguments } }).to_string();
    answer(ctx, &line).expect("a request is answered")["result"].clone()
}

fn ok(ctx: &mut Ctx, name: &str, arguments: Value) -> Value {
    let r = reply(ctx, name, arguments)["structuredContent"].clone();
    assert_eq!(r["ok"], json!(true), "{name}: {r}");
    r
}

/// The plate 60 x 40 x 4: its four vertical edges rounded 5, four counterbored holes from a sketch on its top.
fn plate(ctx: &mut Ctx) {
    let _ = ok(ctx, "box", json!({ "x": 60, "y": 40, "z": 4 }));
    let side = |axis: &str, max: bool| json!({ "extreme": { "axis": axis, "max": max } });
    let corner = |x: bool, y: bool| json!({ "between": { "one": side("x", x), "other": side("y", y) } });
    let rounded = ok(ctx, "fillet", json!({ "edges": { "union": [corner(false, false), corner(false, true), corner(true, false), corner(true, true)] }, "radius": 5 }));
    let top = ok(ctx, "create_sketch", json!({ "plane": { "body": { "part": rounded["bodies"][0]["part"] }, "face": "top" } }));
    let points: Vec<Value> = [[-22, -12], [22, -12], [-22, 12], [22, 12]].iter().map(|p| json!({ "point": { "at": p } })).collect();
    let _ = ok(ctx, "sketch_add", json!({ "sketch": top["sketch"], "entities": points }));
    let _ = ok(ctx, "hole", json!({ "sketch": top["sketch"], "diameter": 3.4, "depth": 10, "kind": "counterbore", "recess_diameter": 6.5, "recess_depth": 2 }));
}

/// A tube: a cylinder 12 round, 30 high, with a 7 bore through it.
fn tube(ctx: &mut Ctx) {
    let _ = ok(ctx, "cylinder", json!({ "radius": 12, "height": 30 }));
    let _ = ok(ctx, "hole", json!({ "face": "top", "diameter": 14, "depth": 40 }));
}

/// A tray: a block 50 x 30 x 15 shelled 2 inward, open at the top, the edges of its bottom rounded 1.
fn tray(ctx: &mut Ctx) {
    let _ = ok(ctx, "box", json!({ "x": 50, "y": 30, "z": 15 }));
    let _ = ok(ctx, "shell", json!({ "thickness": 2, "open": "top" }));
    let _ = ok(ctx, "fillet", json!({ "edges": { "adjacent": "bottom" }, "radius": 1 }));
}

/// A part to draw: its name and how it is made.
struct Part {
    name: &'static str,
    make: fn(&mut Ctx),
}

const PARTS: [Part; 3] = [Part { name: "plate", make: plate }, Part { name: "tube", make: tube }, Part { name: "tray", make: tray }];
const VIEWS: [&str; 2] = ["iso", "top"];

fn kept(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(format!("{name}.png"))
}

/// The pixels of a PNG as RGBA.
fn pixels(png: &[u8]) -> image::RgbaImage {
    image::load_from_memory_with_format(png, image::ImageFormat::Png).expect("a PNG").to_rgba8()
}

/// The PNG a reply carries.
fn png_of(reply: &Value) -> Vec<u8> {
    let image = reply["content"].as_array().and_then(|c| c.iter().find(|c| c["type"] == "image")).unwrap_or_else(|| panic!("no image in {reply}"));
    decode(image["data"].as_str().expect("data"))
}

fn decode(text: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        _ => 63,
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

/// The share of the pixels that differ by more than 16 in any channel: a shade a hair apart is the same pixel.
fn differing(a: &image::RgbaImage, b: &image::RgbaImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions(), "the pictures are not one size");
    let apart = a.pixels().zip(b.pixels()).filter(|(p, q)| p.0.iter().zip(q.0.iter()).any(|(x, y)| x.abs_diff(*y) > 16)).count();
    apart as f64 / (a.width() * a.height()) as f64
}

#[test]
fn every_part_looks_as_it_was_looked_at() {
    let bless = std::env::var("QYM_BLESS").is_ok_and(|v| v == "1");
    let mut wrong = Vec::new();
    for part in &PARTS {
        let mut ctx = Ctx::blank();
        (part.make)(&mut ctx);
        for view in VIEWS {
            let name = format!("{}-{view}", part.name);
            let png = png_of(&reply(&mut ctx, "render", json!({ "view": view, "width": 400, "height": 300 })));
            let path = kept(&name);
            if bless {
                std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder");
                std::fs::write(&path, &png).expect("written");
                continue;
            }
            let Ok(want) = std::fs::read(&path) else {
                wrong.push(format!("{name}: no kept picture at {}", path.display()));
                continue;
            };
            let share = differing(&pixels(&png), &pixels(&want));
            if share >= 0.01 {
                let seen = std::env::temp_dir().join(format!("qym-render-{name}.png"));
                let _ = std::fs::write(&seen, &png);
                wrong.push(format!("{name}: {:.2} % of the pixels differ; drawn now: {}", share * 100.0, seen.display()));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
