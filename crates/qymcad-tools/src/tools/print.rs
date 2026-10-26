//! READY FOR A PRINTER: what a body's mesh is made of, as a slicer meets it. Nothing is changed.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::BodyRef;
use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};
use crate::tools::features::{body_field, target};
use crate::tools::query::round;

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
        let r = qymcad_doc::print::mesh_report::mesh_report(mesh);
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert("triangles".into(), json!(r.triangles));
        out.insert("dropped".into(), json!(r.dropped));
        out.insert("holes".into(), json!(r.holes));
        out.insert("open_edges".into(), json!(r.open_edges));
        out.insert("widest_hole".into(), json!(round(r.widest_hole)));
        out.insert("non_manifold".into(), json!(r.non_manifold));
        out.insert("flipped".into(), json!(r.flipped));
        out.insert("pieces".into(), json!(r.pieces));
        out.insert("closed".into(), json!(r.closed));
        out.insert("volume".into(), json!(round(r.volume)));
        out.insert("area".into(), json!(round(r.area)));
        Ok(out)
    },
};
