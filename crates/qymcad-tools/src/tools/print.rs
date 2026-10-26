//! READY FOR A PRINTER: what a body's mesh is made of, as a slicer meets it. Nothing is changed.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::BodyRef;
use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};
use crate::tools::features::{body_field, target};
use crate::tools::query::round;
use qymcad_doc::print::mesh_repair::{mesh_repair, Repair};
use qymcad_doc::print::mesh_report::{mesh_report, MeshReport};

/// A mesh's health as the answer tells it.
fn report_json(r: &MeshReport) -> Value {
    json!({
        "triangles": r.triangles,
        "dropped": r.dropped,
        "holes": r.holes,
        "open_edges": r.open_edges,
        "widest_hole": round(r.widest_hole),
        "non_manifold": r.non_manifold,
        "flipped": r.flipped,
        "pieces": r.pieces,
        "closed": r.closed,
        "volume": round(r.volume),
        "area": round(r.area),
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyArgs {
    body: Option<BodyRef>,
}

pub const MESH_REPORT: Tool = Tool {
    name: "mesh_report",
    description: "The health of a body's mesh as a slicer meets it, before it is printed or repaired: triangles, dropped (of no use - no area, a repeat, a corner that is no number), holes (borders the mesh is open along) with open_edges and widest_hole (the longest border, mm: a hole of 3 edges is a missing triangle, a long one an open side), non_manifold (edges three triangles share), flipped (triangles wound inward), pieces (parts no triangle joins, printed apart), closed (no hole and no edge of three), volume (mm^3, of no meaning when open) and area (mm^2). A body brought in by import_mesh is read as it came; a body of faces as its triangles in the window. Nothing is changed.",
    schema: || json!({ "type": "object", "properties": { "body": body_field() }, "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: BodyArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let mesh = project
            .mesh_index(body)
            .map(|i| &project.bodies[i].mesh)
            .filter(|m| !m.tris.is_empty())
            .ok_or_else(|| Refusal::new("no-mesh", &format!("Body {body} has no triangles to read."), Stage::Resolve).with_hint("Rebuild the document, or name another body."))?;
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        if let Value::Object(read) = report_json(&mesh_report(mesh)) {
            out.extend(read);
        }
        Ok(out)
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairArgs {
    body: Option<BodyRef>,
    #[serde(default = "sixty_four")]
    fill_holes: usize,
    #[serde(default)]
    drop_under: f64,
    #[serde(default)]
    simplify: f64,
}

fn sixty_four() -> usize {
    64
}

pub const MESH_REPAIR: Tool = Tool {
    name: "mesh_repair",
    description: "Mend a mesh brought in by import_mesh for a slicer, as one step undo takes back: corners that coincide welded, triangles of no use dropped, every triangle wound outward, each hole of up to fill_holes edges (64 by default, 0 none) closed by a fan from the middle of its border, pieces closing less than drop_under mm^3 dropped (0, none, by default), and the mesh made lighter within simplify mm (0 by default; it drops the colours a file gave single triangles). A wider hole is left open and counted: closing an open side by a guess makes a lid nobody drew. The answer holds what was done and mesh_report's numbers before and after. A body of faces is closed by the kernel already, and a mesh a later step took in (recognise_mesh) is refused as that body is.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "fill_holes": { "type": "integer", "minimum": 0, "maximum": 100000, "default": 64, "description": "Close each hole of up to this many edges." },
            "drop_under": { "type": "number", "minimum": 0, "description": "Drop the pieces closing less than this many mm^3." },
            "simplify": { "type": "number", "minimum": 0, "maximum": 10, "description": "Make the mesh lighter within this deviation, mm." },
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: RepairArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let piece = project.timeline.iter().any(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::MeshPiece { body: b, .. } if b == body));
        if !piece {
            return Err(Refusal::new("not-a-mesh", &format!("Body {body} is no mesh brought in: a body of faces is closed by the kernel already."), Stage::Validate).with_hint("Name a body brought in by import_mesh; mesh_report reads any body."));
        }
        let Some(mesh) = project.mesh_index(body).map(|i| project.bodies[i].mesh.clone()) else {
            return Err(Refusal::new("no-mesh", &format!("Body {body} has no triangles to mend."), Stage::Resolve));
        };
        let before = mesh_report(&mesh);
        let mended = mesh_repair(&mesh, &Repair { drop_under: a.drop_under, fill_up_to: a.fill_holes, simplify: a.simplify });
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert("closed_holes".into(), json!(mended.closed_holes));
        out.insert("left_holes".into(), json!(mended.left_holes));
        out.insert("dropped_pieces".into(), json!(mended.dropped_pieces));
        out.insert("before".into(), report_json(&before));
        out.insert("after".into(), report_json(&mesh_report(&mended.mesh)));
        ctx.doc.edit("status-mesh-repair", |p| if p.replace_piece_mesh(body, mended.into_piece()) { Ok(()) } else { Err("not-a-mesh".to_string()) }).map_err(tool::doc_refusal)?;
        Ok(out)
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckArgs {
    body: Option<BodyRef>,
    printer: Option<String>,
    bed: Option<[f64; 3]>,
    #[serde(default = "forty_five")]
    overhang: f64,
    #[serde(default = "point_four")]
    nozzle: f64,
    wall: Option<f64>,
}

fn forty_five() -> f64 {
    45.0
}

fn point_four() -> f64 {
    0.4
}

/// The points the walls are read at: enough for a wall of a few mm^2 to be met, few enough for a mesh of a million
/// triangles to be read in 0.15 s (measured, release, 20 000 points).
const WALL_POINTS: usize = 20_000;

/// The room a printer builds in when none is named: the most common room of the printers known.
const ASSUMED: [f64; 3] = [256.0, 256.0, 256.0];

/// A verdict as the answer tells it.
fn said(v: qymcad_doc::print::checks::Verdict) -> &'static str {
    use qymcad_doc::print::checks::Verdict;
    match v {
        Verdict::Pass => "pass",
        Verdict::Warn => "warn",
        Verdict::Fail => "fail",
    }
}

pub const CHECK_PRINTABILITY: Tool = Tool {
    name: "check_printability",
    description: "Whether a body prints as it lies, read where a slicer drops it - onto the bed through its lowest point: closed (no hole, no edge of three triangles; mesh_repair mends one brought in), valid (the kernel finds a body of faces sound), bed (it fits the room the printer builds in as it lies, or only turned a quarter about Z), contact (the area of its faces flat on the bed - none is a body on an edge or a point), overhang (the area facing down past overhang deg from the upright, 45 by default, the bottom left out, and up to 10 places it hangs at, the widest first - give them to render marks to see them), walls (the thinnest wall read by rays from 20 000 points into the material, against wall - two lines of nozzle, 0.8 for the nozzle of 0.4 by default - and up to 10 places thinner, the thinnest first), small (round faces of a radius under the nozzle's - a hole that closes up, a pin thinner than a line - with their face key, and edges shorter than the nozzle; read on exact faces, null on a mesh brought in, said by the warning small-unread-on-mesh). Each check says pass, warn or fail with its numbers; verdict is the worst. printer names a printer by its room (p1s for X1C, P1P, P1S, P2S and A1; a1_mini, h2d for both nozzles, h2d_one, h2s, h2c, x2d; k1c, k2, k1_max, k2_plus, ender3_v3_plus; centauri, neptune4_max; u1), or bed gives the room [x, y, z] mm; with neither 256 x 256 x 256 is taken and said so. Nothing is changed.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "printer": { "type": "string", "description": "A printer by its room: p1s, a1_mini, h2d, h2d_one, h2s, h2c, x2d, k1c, k2, k1_max, k2_plus, ender3_v3_plus, centauri, neptune4_max, u1, or one of the person's own printers.json." },
            "bed": { "type": "array", "items": { "type": "number", "exclusiveMinimum": 0 }, "minItems": 3, "maxItems": 3, "description": "The room the printer builds in, [x, y, z] mm." },
            "overhang": { "type": "number", "minimum": 0, "maximum": 89, "default": 45, "description": "Degrees from the upright a face may turn down unsupported." },
            "nozzle": { "type": "number", "exclusiveMinimum": 0, "maximum": 2, "default": 0.4, "description": "The nozzle's bore, mm: 0.2, 0.4, 0.6, 0.8." },
            "wall": { "type": "number", "exclusiveMinimum": 0, "description": "The thinnest wall that prints, mm; two lines of the nozzle when left out." },
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        use qymcad_doc::print::checks::{contact, fits, overhang, Bed, Fit, Verdict};
        let a: CheckArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let mut warnings = Vec::new();
        let (printer, build) = match (&a.printer, a.bed) {
            (Some(_), Some(_)) => return Err(Refusal::new("arguments", "Give printer or bed, one of the two.", Stage::Validate)),
            (Some(name), None) => match crate::printers::named(name) {
                Some(p) => (json!(p.name), p.build),
                None => {
                    let known: Vec<String> = crate::printers::all().into_iter().map(|p| p.name).collect();
                    return Err(Refusal::new("no-printer", &format!("No printer is known as {name:?}."), Stage::Validate).with_hint(&format!("Name one of {}, or give bed: [x, y, z] mm.", known.join(", "))));
                }
            },
            (None, Some(bed)) => (Value::Null, bed),
            (None, None) => {
                warnings.push(json!("bed-assumed"));
                (Value::Null, ASSUMED)
            }
        };
        let mesh = qymcad_doc::print::world_mesh(project, body)
            .filter(|m| !m.tris.is_empty())
            .ok_or_else(|| Refusal::new("no-mesh", &format!("Body {body} has no triangles to read."), Stage::Resolve).with_hint("Rebuild the document, or name another body."))?;
        let health = mesh_report(&mesh);
        let closed = if health.closed { Verdict::Pass } else { Verdict::Fail };
        let sound = ctx.doc.shape(body).map(|s| s.is_valid());
        let valid = match sound {
            Some(false) => Verdict::Fail,
            _ => Verdict::Pass,
        };
        let on = fits(&mesh, &Bed { size: build });
        let touch = contact(&mesh);
        let hang = overhang(&mesh, a.overhang);
        if !(a.nozzle > 0.0 && a.wall.is_none_or(|w| w > 0.0)) {
            return Err(Refusal::new("arguments", "A nozzle and a wall are sizes over 0 mm.", Stage::Validate));
        }
        let least = a.wall.unwrap_or(2.0 * a.nozzle);
        let walls = qymcad_doc::print::thickness::thickness(&mesh, &qymcad_doc::print::thickness::Walls { least, samples: WALL_POINTS });
        // what is too small is read on exact faces: a mesh brought in has none, and says so
        let small = ctx.doc.shape(body).map(|shape| qymcad_doc::print::small::small_features(project, shape, body, a.nozzle));
        if small.is_none() {
            warnings.push(json!("small-unread-on-mesh"));
        }
        let small_verdict = small.as_ref().map_or(Verdict::Pass, |s| s.verdict);
        let worst = [closed, valid, on.verdict, touch.verdict, hang.verdict, walls.verdict, small_verdict].into_iter().max().unwrap_or(Verdict::Pass);
        let fit = match on.fit {
            Fit::AsItLies => "as_it_lies",
            Fit::Turned => "turned",
            Fit::Not => "no",
        };
        let places: Vec<Value> = hang.places.iter().map(|p| json!({ "at": p.at.map(round), "area": round(p.area) })).collect();
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert("verdict".into(), json!(said(worst)));
        out.insert("printer".into(), printer);
        out.insert("build".into(), json!(build));
        out.insert(
            "checks".into(),
            json!({
                "closed": { "verdict": said(closed), "holes": health.holes, "non_manifold": health.non_manifold },
                "valid": { "verdict": said(valid), "valid": sound },
                "bed": { "verdict": said(on.verdict), "fit": fit, "size": on.size.map(round) },
                "contact": { "verdict": said(touch.verdict), "area": round(touch.area) },
                "overhang": { "verdict": said(hang.verdict), "angle": a.overhang, "area": round(hang.area), "places": places },
                "small": small.as_ref().map(|s| json!({
                    "verdict": said(s.verdict),
                    "rounds": s.rounds.iter().map(|r| json!({ "at": r.at.map(round), "radius": round(r.radius), "kind": match r.kind { qymcad_doc::print::small::Round::Hole => "hole", qymcad_doc::print::small::Round::Pin => "pin" }, "face": r.face })).collect::<Vec<Value>>(),
                    "edges": s.edges.iter().map(|e| json!({ "at": e.at.map(round), "length": round(e.length) })).collect::<Vec<Value>>(),
                })),
                "walls": {
                    "verdict": said(walls.verdict),
                    "least": if walls.least.is_finite() { json!(round(walls.least)) } else { Value::Null },
                    "required": least,
                    "nozzle": a.nozzle,
                    "places": walls.places.iter().map(|p| json!({ "at": p.at.map(round), "thickness": round(p.thickness) })).collect::<Vec<Value>>(),
                },
            }),
        );
        if !warnings.is_empty() {
            out.insert("warnings".into(), json!(warnings));
        }
        Ok(out)
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaceArgs {
    body: Option<BodyRef>,
    face: Option<crate::args::QueryArg>,
    down: Option<[f64; 3]>,
}

pub const PLACE_ON_FACE: Tool = Tool {
    name: "place_on_face",
    description: "Lay a body on one of its faces for printing: the face turns to look straight down, about the middle of the body, and the body's lowest point goes to z = 0. face is a query finding one flat face (\"bottom\" lays it as it stands; a key from list_faces as {\"ids\": [key]}); or down gives the way the face looks now, in the world - the normal point_at reads off a picture, for a body brought in by import_mesh, whose faces have no keys to query. One move node, as move lays it: a body moved before takes this on top. The answer gives the body's size and lowest point after. suggest_orientation tells which face to lay it on.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "face": crate::args::query_schema(),
            "down": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "The way the face looks now, in the world." },
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: PlaceArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let down = match (&a.face, a.down) {
            (Some(query), None) => {
                let found = crate::args::resolve(project, body, &crate::args::reference(query, crate::args::Expect::One), crate::args::Element::Faces)?;
                let faces = project.regen_faces.get(&body).map(Vec::as_slice).or_else(|| project.mesh_index(body).map(|i| project.bodies[i].faces.as_slice())).unwrap_or(&[]);
                let face = faces.iter().find(|f| f.id == found[0]).ok_or_else(|| Refusal::new("no-face", &format!("Face {} of body {body} has no triangles to read its look from.", found[0]), Stage::Resolve))?;
                qymcad_core::feature::apply12_dir(&project.body_world_transform(body), face.normal)
            }
            (None, Some(d)) => d,
            _ => return Err(Refusal::new("arguments", "Give face or down, one of the two.", Stage::Validate)),
        };
        let m = qymcad_doc::print::orient::lay_on(project, body, down).ok_or_else(|| Refusal::new("bad-direction", &format!("The way {down:?} has no length to lay the body by."), Stage::Validate))?;
        let node = ctx
            .doc
            .edit("status-move-body", |p| {
                // a body standing as a move takes this one on top, so two lays are one node
                if p.accumulate_move(body, &m) {
                    Ok(body)
                } else {
                    Ok(p.add_move(body, m))
                }
            })
            .map_err(tool::doc_refusal)?;
        let mut out = crate::tools::modify::laid_node(ctx, node);
        let now = ctx.doc.project().timeline.iter().find(|n| n.id == node).and_then(|n| n.kind.body()).unwrap_or(body);
        if let Some(b) = qymcad_doc::print::world_mesh(ctx.doc.project(), now).and_then(|m| m.bounds()) {
            out.insert("size".into(), json!([b.max.x - b.min.x, b.max.y - b.min.y, b.max.z - b.min.z].map(round)));
            out.insert("lowest".into(), json!(round(b.min.z)));
        }
        Ok(out)
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SuggestArgs {
    body: Option<BodyRef>,
    #[serde(default = "forty_five")]
    overhang: f64,
}

pub const SUGGEST_ORIENTATION: Tool = Tool {
    name: "suggest_orientation",
    description: "The best ways to lay a body for printing, the best first, up to 3: each of its 6 broadest flat faces looking down and each way along the world's axes, rated as it would then print - overhang (mm^2 hanging past overhang deg from the upright, 45 by default; least first), contact (mm^2 on the bed; most next), height (mm; lowest last). A way standing on an edge or a point comes last. Each way holds lay: the arguments place_on_face takes to lay it so. as_it_lies rates the body as it stands now. Nothing is changed.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "overhang": { "type": "number", "minimum": 0, "maximum": 89, "default": 45, "description": "Degrees from the upright a face may turn down unsupported." },
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SuggestArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let mesh = qymcad_doc::print::world_mesh(project, body)
            .filter(|m| !m.tris.is_empty())
            .ok_or_else(|| Refusal::new("no-mesh", &format!("Body {body} has no triangles to read."), Stage::Resolve).with_hint("Rebuild the document, or name another body."))?;
        let ways = qymcad_doc::print::orient::suggest(project, body, a.overhang);
        let told: Vec<Value> = ways
            .iter()
            .enumerate()
            .map(|(k, w)| {
                let lay = match w.face {
                    Some(key) => json!({ "body": { "body": body }, "face": { "ids": [key] } }),
                    None => json!({ "body": { "body": body }, "down": w.down.map(round) }),
                };
                json!({ "rank": k + 1, "down": w.down.map(round), "face": w.face, "overhang": round(w.overhang), "contact": round(w.contact), "height": round(w.height), "lay": lay })
            })
            .collect();
        let height = mesh.bounds().map_or(0.0, |b| b.max.z - b.min.z);
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert("ways".into(), json!(told));
        out.insert(
            "as_it_lies".into(),
            json!({
                "overhang": round(qymcad_doc::print::checks::overhang(&mesh, a.overhang).area),
                "contact": round(qymcad_doc::print::checks::contact(&mesh).area),
                "height": round(height),
            }),
        );
        Ok(out)
    },
};
