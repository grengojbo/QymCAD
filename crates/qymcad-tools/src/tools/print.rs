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
}

fn forty_five() -> f64 {
    45.0
}

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
    description: "Whether a body prints as it lies, read where a slicer drops it - onto the bed through its lowest point: closed (no hole, no edge of three triangles; mesh_repair mends one brought in), valid (the kernel finds a body of faces sound), bed (it fits the room the printer builds in as it lies, or only turned a quarter about Z), contact (the area of its faces flat on the bed - none is a body on an edge or a point), overhang (the area facing down past overhang deg from the upright, 45 by default, the bottom left out, and up to 10 places it hangs at, the widest first - give them to render marks to see them). Each check says pass, warn or fail with its numbers; verdict is the worst. printer names a printer by its room (p1s for X1C, P1P, P1S, P2S and A1; a1_mini, h2d for both nozzles, h2d_one, h2s, h2c, x2d; k1c, k2, k1_max, k2_plus, ender3_v3_plus; centauri, neptune4_max; u1), or bed gives the room [x, y, z] mm; with neither 256 x 256 x 256 is taken and said so. Nothing is changed.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "printer": { "type": "string", "description": "A printer by its room: p1s, a1_mini, h2d, h2d_one, h2s, h2c, x2d, k1c, k2, k1_max, k2_plus, ender3_v3_plus, centauri, neptune4_max, u1, or one of the person's own printers.json." },
            "bed": { "type": "array", "items": { "type": "number", "exclusiveMinimum": 0 }, "minItems": 3, "maxItems": 3, "description": "The room the printer builds in, [x, y, z] mm." },
            "overhang": { "type": "number", "minimum": 0, "maximum": 89, "default": 45, "description": "Degrees from the upright a face may turn down unsupported." },
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
        let worst = [closed, valid, on.verdict, touch.verdict, hang.verdict].into_iter().max().unwrap_or(Verdict::Pass);
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
            }),
        );
        if !warnings.is_empty() {
            out.insert("warnings".into(), json!(warnings));
        }
        Ok(out)
    },
};
