//! A SECTION OF A BODY OF A MESH: read as the lines, arcs, circles and splines it is fitted with, or drawn into a sketch.
use qymcad_core::feature::BasePlane;
use qymcad_core::geom::{Fitted, Piece};
use qymcad_core::model::{Id, OfSection, Project, SectionAt};
use qymcad_meshfit::section::{fit_section, Within};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::{amount_schema, Amount, BodyRef};
use crate::tool::{self, Ctx, Refusal, Stage, Tool};
use crate::tools::doc::{answer, outcome};
use crate::tools::features::body_field;

/// The base plane a section is taken off.
#[derive(Deserialize, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
enum PlaneArg {
    #[default]
    Xy,
    Xz,
    Yz,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SectionArgs {
    body: Option<BodyRef>,
    #[serde(default)]
    plane: PlaneArg,
    offset: Option<Amount>,
    tolerance: Option<f64>,
}

fn schema() -> Value {
    json!({ "type": "object", "properties": {
        "body": body_field(),
        "plane": { "type": "string", "enum": ["xy", "xz", "yz"], "description": "The base plane the section is taken off; xy by default." },
        "offset": amount_schema("How far the plane is moved off the base plane, mm, the way a datum plane off it moves; the middle of the body by default"),
        "tolerance": { "type": "number", "exclusiveMinimum": 0, "description": "How far a corner of the mesh may lie off the line or arc it is taken into, mm; 2e-4 of the size of each loop, never under 0.01, by default." },
    }, "additionalProperties": false })
}

/// What a call asks for, read: the body, where it is cut, how finely.
struct Asked {
    body: Id,
    at: SectionAt,
    within: Within,
}

fn asked(project: &Project, a: SectionArgs) -> Result<Asked, Refusal> {
    let body = match a.body {
        Some(b) => b.resolve(project)?,
        None => BodyRef::Part(project.current_ctx()).resolve(project).map_err(|r| r.with_hint("Name the body: {\"body\": key} or {\"part\": key}."))?,
    };
    let base = match a.plane {
        PlaneArg::Xy => BasePlane::XY,
        PlaneArg::Xz => BasePlane::XZ,
        PlaneArg::Yz => BasePlane::YZ,
    };
    let extent = project.extent_along(body, base).ok_or_else(|| Refusal::new("no-mesh", "The body has no mesh to cut.", Stage::Validate))?;
    let offset = match &a.offset {
        Some(o) => o.read(project, "offset")?.value,
        None => (extent.from + extent.to) / 2.0,
    };
    if offset <= extent.from || offset >= extent.to {
        return Err(Refusal::new("off-the-body", &format!("The plane at {offset} misses the body: it stands from {} to {} along the plane's normal.", extent.from, extent.to), Stage::Validate));
    }
    let within = match a.tolerance {
        Some(t) if t.is_finite() && t > 0.0 => Within::Typed(t),
        Some(_) => return Err(Refusal::new("bad-tolerance", "tolerance must be a number over zero.", Stage::Validate)),
        None => Within::OwnTolerance,
    };
    Ok(Asked { body, at: SectionAt { base, offset }, within })
}

/// A piece in the plane's coordinates, for the answer.
fn piece(p: &Piece) -> Value {
    match p {
        Piece::Line { from, to } => json!({ "kind": "line", "from": [from.x, from.y], "to": [to.x, to.y], "length": from.dist(*to) }),
        Piece::Arc { center, radius, from, to, winding } => {
            json!({ "kind": "arc", "center": [center.x, center.y], "radius": radius, "from": [from.x, from.y], "to": [to.x, to.y], "ccw": *winding == qymcad_core::feature::Winding::Ccw })
        }
        Piece::Circle { center, radius } => json!({ "kind": "circle", "center": [center.x, center.y], "radius": radius }),
        Piece::Spline(points) => json!({ "kind": "spline", "points": points.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>() }),
    }
}

fn loops(found: &[Fitted]) -> Value {
    json!(found.iter().map(|l| json!({ "pieces": l.pieces.iter().map(piece).collect::<Vec<_>>() })).collect::<Vec<_>>())
}

pub const MESH_SECTION: Tool = Tool {
    name: "mesh_section",
    description: "Read the section of a body's mesh by a plane, fitted with lines, arcs, circles and splines - what Sketch from section would draw, nothing laid. plane is the base plane (xy, xz, yz), offset how far it is moved along its normal (the middle of the body by default), tolerance how far a corner may lie off its line or arc. The answer gives the frame of the plane (origin, x, y in world mm) and each closed loop's pieces in the plane's coordinates: a line its ends and length, an arc its centre, radius, ends and turn, a circle its centre and radius, a spline its points.",
    schema,
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SectionArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let Asked { body, at, within } = asked(project, a)?;
        let mesh = &project.bodies.iter().find(|b| b.id == body).ok_or_else(|| Refusal::new("no-mesh", "The body has no mesh to cut.", Stage::Validate))?.mesh;
        let frame = at.frame();
        let found = fit_section(mesh, &frame, within);
        let mut out = answer("body", json!(body));
        out.insert("frame".into(), json!({ "origin": frame.origin, "x": frame.x, "y": frame.y }));
        out.insert("offset".into(), json!(at.offset));
        out.insert("loops".into(), loops(&found));
        Ok(out)
    },
};

pub const SKETCH_FROM_SECTION: Tool = Tool {
    name: "sketch_from_section",
    description: "Draw the section of a body of a mesh (an STL, OBJ or 3MF brought in, or a polyhedron made of one) into a sketch: the plane cuts the mesh (plane, offset, tolerance as mesh_section), the loops fitted with lines, arcs, circles and splines become curves of their own, given the relations they show and sizes until fully defined, measured from the origin, rounded to a tenth. Where the offset is not zero a datum plane is laid there first and the sketch stands on it. Extrude or revolve the sketch to have the part again. A body of exact surfaces is refused: take its outline with sketch_from_face. The answer says what the sketch holds: constraints, sizes, freedoms left, sizes rounded, and the pieces counted.",
    schema,
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SectionArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let Asked { body, at, within } = asked(project, a)?;
        if !project.came_from_a_mesh(body) {
            return Err(Refusal::new("exact-body", "This body is made of exact surfaces, not of a mesh.", Stage::Validate).with_hint("Take its outline with sketch_from_face, or project its edges into a sketch."));
        }
        let name = qymcad_i18n::tr1("g-sketch-n", "n", &(project.sketches.len() + 1).to_string());
        let fit = move |m: &qymcad_core::geom::Mesh, f: &qymcad_core::feature::PlaneFrame| fit_section(m, f, within);
        let made = ctx
            .doc
            .edit("f-sketch-from-section", |p| {
                let part = p.body_owner(body);
                p.set_active_component(part);
                p.add_sketch_of_section(OfSection { body, at, fit: &fit, name }).ok_or_else(|| "msg-sketch-from-section-misses".to_string())
            })
            .map_err(tool::doc_refusal)?;
        let d = &made.defined;
        let held = json!({
            "id": made.sketch,
            "constraints": d.relations,
            "sizes": d.sizes,
            "free": d.free,
            "rounded": d.rounded.iter().map(|r| json!({ "was": r.was, "now": r.now })).collect::<Vec<_>>(),
        });
        let mut out = outcome(ctx, answer("sketch", json!(made.sketch)));
        out.insert("held".into(), held);
        Ok(out)
    },
};
