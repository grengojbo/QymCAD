//! ASKING THE MODEL: the faces and edges of a body with their keys, what a query finds, the health of a body, and a
//! measurement between faces, edges and points - all read, nothing changed, so none of them is an undo step.
//!
//! Every place and direction is in world coordinates, the frame `render` draws and `measure` measures in; the keys
//! are the ones `ids` queries and the features take.

use serde::Deserialize;
use serde_json::{json, Value};

use qymcad_core::measure::{measure_one, measure_pair, MeasureItem, MeasureResult};
use qymcad_core::model::{Id, Project};

use crate::args::{self, BodyRef, Element, Expect, QueryArg};
use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};
use crate::tools::features::{body_field, target};

/// A point placed by `wt`, rounded to a micrometre: the kernel's last digits are noise a reader would take for a size.
fn at(wt: &[f64; 12], p: [f64; 3]) -> [f64; 3] {
    qymcad_core::feature::apply12(wt, p).map(round)
}

fn dir(wt: &[f64; 12], d: [f64; 3]) -> [f64; 3] {
    qymcad_core::feature::apply12_dir(wt, d).map(round)
}

fn round(v: f64) -> f64 {
    let r = (v * 1e6).round() / 1e6;
    if r == 0.0 {
        0.0 // no -0
    } else {
        r
    }
}

/// The live shape of a body: the shape the rebuild left, which every reading here asks.
fn shape(ctx: &Ctx, body: Id) -> Result<&qymcad_kernel::Shape, Refusal> {
    ctx.doc.shape(body).ok_or_else(|| Refusal::new("no-body", &format!("Body {body} has no shape: it did not build."), Stage::Validate).with_hint("Read get_document for what went red."))
}

/// WHO MADE A FACE: the feature the document's name table records as its creator, not the last one that touched it;
/// its kind, the role the face plays in it, how many faces of `body` it made, and its sizes with the expressions they
/// follow. A face with no structural name (a solid from a file, a face the kernel numbered by position) has no
/// recipe to tell, and answers `null` rather than a guess.
fn face_made_by(project: &Project, body: Id, face: u32) -> Value {
    let Some(name) = project.names.get(face) else { return Value::Null };
    author(project, body, name.feature, json!(format!("{:?}", name.role)))
}

/// WHO MADE AN EDGE: the later, in the timeline, of the creators of the two faces it lies between - the rim of a
/// rounding lies between the rounding and the wall it runs into, and it is the rounding that put it there. The edge
/// has no role of its own.
fn edge_made_by(project: &Project, body: Id, edge: u32) -> Value {
    let Some(name) = project.names.edge(edge) else { return Value::Null };
    let place = |feature: Id| project.timeline.iter().position(|n| n.id == feature);
    let later = name.faces.iter().filter_map(|&f| project.names.get(f)).filter_map(|n| place(n.feature).map(|at| (at, n.feature))).max_by_key(|(at, _)| *at);
    match later {
        Some((_, feature)) => author(project, body, feature, Value::Null),
        None => Value::Null,
    }
}

/// The account of `feature` as the author of a face or an edge of `body`.
fn author(project: &Project, body: Id, feature: Id, role: Value) -> Value {
    let Some(node) = project.timeline.iter().find(|n| n.id == feature) else { return Value::Null };
    let faces_made = project.regen_faces.get(&body).map_or(0, |faces| faces.iter().filter(|f| project.names.get(f.id).is_some_and(|n| n.feature == feature)).count());
    json!({
        "feature": feature,
        "kind": qymcad_doc::report::kind_of(&node.kind),
        "role": role,
        "faces_made": faces_made,
        "sizes": crate::tools::timeline::sizes(project, feature),
    })
}

/// WHAT KIND OF SURFACE A FACE LIES ON, from what the model and the kernel already tell: a cylinder when the kernel
/// gives its radius, a plane when every triangle of the face faces one way, a turned surface (a cone, a sphere, a
/// torus) when it has an axis, and otherwise another.
fn face_kind(project: &Project, shape: &qymcad_kernel::Shape, body: Id, face: &qymcad_core::geom::MeshFace) -> &'static str {
    if shape.face_cylinder(face.id).is_some() {
        return "cylinder";
    }
    let flat = project.mesh_index(body).map(|mi| &project.bodies[mi].mesh).is_some_and(|mesh| {
        face.triangles.iter().all(|&t| {
            let [a, b, c] = mesh.triangle(t as usize);
            let (u, v) = ([b.x - a.x, b.y - a.y, b.z - a.z], [c.x - a.x, c.y - a.y, c.z - a.z]);
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            // a sliver triangle has no direction to disagree with
            l < 1e-12 || (n[0] * face.normal[0] + n[1] * face.normal[1] + n[2] * face.normal[2]) / l > 1.0 - 1e-6
        })
    });
    if flat {
        "plane"
    } else if shape.face_axis(face.id).is_some() {
        "turned"
    } else {
        "other"
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BodyArgs {
    body: Option<BodyRef>,
}

fn body_only() -> Value {
    json!({ "type": "object", "properties": { "body": body_field() }, "additionalProperties": false })
}

pub const LIST_FACES: Tool = Tool {
    name: "list_faces",
    description: "The faces of a body: key, kind (plane, cylinder, turned - a cone, sphere or torus - or other), centre, normal, area in mm^2, and for a cylinder its radius and axis. made_by: the feature that created the face (key, kind, the role of the face, how many faces of the body it made, its sizes with their expressions) - edit_feature on that key changes it; null for a face with no recipe, as from a STEP file. World coordinates. A key goes into a query as {\"ids\": [key]}.",
    schema: body_only,
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: BodyArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let shape = shape(ctx, body)?;
        let wt = project.body_world_transform(body);
        let faces = project.regen_faces.get(&body).cloned().unwrap_or_default();
        let listed: Vec<Value> = faces
            .iter()
            .map(|f| {
                let mut v = json!({
                    "key": f.id,
                    "kind": face_kind(project, shape, body, f),
                    "centre": at(&wt, [f.centroid.x, f.centroid.y, f.centroid.z]),
                    "normal": dir(&wt, f.normal),
                    "area": round(f.area),
                    "made_by": face_made_by(project, body, f.id),
                });
                if let Some((o, axis, r)) = shape.face_cylinder(f.id) {
                    v["radius"] = json!(round(r));
                    v["axis"] = json!({ "origin": at(&wt, o), "dir": dir(&wt, axis) });
                }
                v
            })
            .collect();
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert("faces".into(), json!(listed));
        Ok(out)
    },
};

pub const LIST_EDGES: Tool = Tool {
    name: "list_edges",
    description: "The edges of a body: key, ends a and b, middle, length in mm, and for a circular edge (the rim of a hole, a rounded corner) its centre, axis and radius. made_by: the later of the features that created its two faces, as list_faces tells it. World coordinates. A key goes into a query as {\"ids\": [key]}.",
    schema: body_only,
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: BodyArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let _ = shape(ctx, body)?;
        let wt = project.body_world_transform(body);
        let listed: Vec<Value> = project
            .edge_pool(body)
            .iter()
            .map(|c| {
                let mut v = json!({ "key": c.desc, "middle": at(&wt, c.centroid), "length": round(c.area), "made_by": edge_made_by(project, body, c.desc) });
                if let Some(e) = &c.edge {
                    v["a"] = json!(at(&wt, e.a));
                    v["b"] = json!(at(&wt, e.b));
                    if e.radius > 0.0 {
                        v["circle"] = json!({ "centre": at(&wt, e.center), "axis": dir(&wt, e.axis), "radius": round(e.radius) });
                    }
                }
                v
            })
            .collect();
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert("edges".into(), json!(listed));
        Ok(out)
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveArgs {
    body: Option<BodyRef>,
    faces: Option<QueryArg>,
    edges: Option<QueryArg>,
    #[serde(default = "any")]
    expect: Expect,
}

fn any() -> Expect {
    Expect::Any
}

pub const RESOLVE: Tool = Tool {
    name: "resolve",
    description: "What a query finds on a body now, before a feature is laid on it: faces or edges (one of the two) as the query a fillet, a shell or a sketch would take; the keys found and how many. expect (any by default) refuses as a feature would: one - exactly one, some - at least one.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "faces": args::query_schema(),
            "edges": args::query_schema(),
            "expect": args::expect_schema(),
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ResolveArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let (query, of) = match (&a.faces, &a.edges) {
            (Some(q), None) => (q, Element::Faces),
            (None, Some(q)) => (q, Element::Edges),
            _ => return Err(Refusal::new("arguments", "Give faces or edges, one of the two.", Stage::Validate)),
        };
        let found = args::resolve(project, body, &args::reference(query, a.expect), of)?;
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert(of.word().into(), json!(found));
        out.insert("count".into(), json!(found.len()));
        Ok(out)
    },
};

/// The kinds of surface the kernel counts, in its order.
const SURFACES: [&str; 7] = ["plane", "cylinder", "cone", "sphere", "torus", "free", "other"];

pub const INSPECT: Tool = Tool {
    name: "inspect",
    description: "The health of a body before it is printed or exported: valid (the kernel finds it sound), solids (1 for one closed body; more means it fell apart, 0 a sheet), its faces by the kind of surface, the smallest radius of a round face on it - a rounding, or the wall of a hole (null when it has none; list_faces tells which). Volume, area and bounds are in get_document.",
    schema: body_only,
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: BodyArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let shape = shape(ctx, body)?;
        let kinds = shape.face_kinds().map(|k| SURFACES.iter().zip(k).filter(|(_, n)| *n > 0).map(|(name, n)| ((*name).to_string(), json!(n))).collect::<serde_json::Map<_, _>>());
        let round_min = shape.min_round_radius();
        let mut out = Answer::new();
        out.insert("body".into(), json!(body));
        out.insert("valid".into(), json!(shape.is_valid()));
        out.insert("solids".into(), json!(shape.solid_count()));
        out.insert("surfaces".into(), json!(kinds));
        out.insert("smallest_radius".into(), if round_min > 0.0 { json!(round(round_min)) } else { Value::Null });
        Ok(out)
    },
};

/// One end of a measurement: a face or an edge a query finds exactly one of, or a point.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    body: Option<BodyRef>,
    face: Option<QueryArg>,
    edge: Option<QueryArg>,
    point: Option<[f64; 3]>,
}

fn target_schema() -> Value {
    json!({ "type": "object", "description": "{\"face\": query} or {\"edge\": query} finding one (with \"body\" unless it is the active part's), or {\"point\": [x, y, z]}.", "properties": {
        "body": body_field(),
        "face": args::query_schema(),
        "edge": args::query_schema(),
        "point": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
    }, "additionalProperties": false })
}

/// What a target is, placed in the world.
fn item(ctx: &Ctx, t: &Target) -> Result<MeasureItem, Refusal> {
    let project = ctx.doc.project();
    let (query, of) = match (&t.face, &t.edge, t.point) {
        (None, None, Some(p)) => return Ok(MeasureItem::Point(p)),
        (Some(q), None, None) => (q, Element::Faces),
        (None, Some(q), None) => (q, Element::Edges),
        _ => return Err(Refusal::new("arguments", "Each end is a face, an edge or a point - one of the three.", Stage::Validate)),
    };
    let body = target(project, t.body)?;
    let found = args::resolve(project, body, &args::reference(query, Expect::One), of)?;
    let shape = shape(ctx, body)?;
    let wt = project.body_world_transform(body);
    match of {
        Element::Faces => Ok(qymcad_doc::measure::face_item(project, Some(shape), body, found[0], &wt)),
        Element::Edges => qymcad_doc::measure::edge_item(shape, found[0], &wt).ok_or_else(|| Refusal::new("no-edge", &format!("Edge {} has no points to measure.", found[0]), Stage::Resolve)),
    }
}

/// An item as the answer names it.
fn named(i: &MeasureItem) -> Value {
    match *i {
        MeasureItem::Point(p) => json!({ "point": p.map(round) }),
        MeasureItem::Line { origin, dir, len } => json!({ "line": { "from": origin.map(round), "dir": dir.map(round), "length": round(len) } }),
        MeasureItem::Circle { center, axis, r } => json!({ "circle": { "centre": center.map(round), "axis": axis.map(round), "radius": round(r) } }),
        MeasureItem::Plane { origin, normal } => json!({ "plane": { "point": origin.map(round), "normal": normal.map(round) } }),
        MeasureItem::Cylinder { origin, axis, r } => json!({ "cylinder": { "point": origin.map(round), "axis": axis.map(round), "radius": round(r) } }),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeasureArgs {
    a: Target,
    b: Option<Target>,
}

pub const MEASURE: Tool = Tool {
    name: "measure",
    description: "Measure, as the window's measuring tool does. One end: the length of an edge, the diameter of a circle or a cylinder. Two ends: the shortest distance where it is one (parallel faces, a point, a cylinder's wall), the angle between their directions (0..90 deg), and between two points the distance along each axis. Each end is {\"face\": query}, {\"edge\": query} or {\"point\": [x, y, z]}; a query must find exactly one. World coordinates.",
    schema: || json!({ "type": "object", "properties": { "a": target_schema(), "b": target_schema() }, "required": ["a"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let m: MeasureArgs = tool::args(arguments)?;
        let a = item(ctx, &m.a)?;
        let b = m.b.as_ref().map(|t| item(ctx, t)).transpose()?;
        let MeasureResult { distance, angle_deg, delta, value } = match &b {
            None => measure_one(&a),
            Some(b) => measure_pair(&a, b),
        };
        let mut out = Answer::new();
        out.insert("a".into(), named(&a));
        if let Some(b) = &b {
            out.insert("b".into(), named(b));
        }
        if let Some(d) = distance {
            out.insert("distance".into(), json!(round(d)));
        }
        if let Some(g) = angle_deg {
            out.insert("angle".into(), json!(round(g)));
        }
        if let Some(d) = delta {
            out.insert("delta".into(), json!(d.map(round)));
        }
        // the window's labels are a catalogue key and a sign; the answer says them in words
        if let Some((label, v)) = value {
            let word = if label == "m3-length" { "length" } else { "diameter" };
            out.insert(word.into(), json!(round(v)));
        }
        Ok(out)
    },
};
