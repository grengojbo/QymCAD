//! FEATURES FROM A SKETCH: an extrusion and a revolution, laid as the window lays them - the feature goes into the
//! part that owns the sketch, all the contours taken make one node, a first body is joined into the part's one body,
//! and with a body there the tool joins it, cuts it or meets it in one boolean. One call is one undo step.

use qymcad_core::feature::{Extent, Reach};
use qymcad_core::model::{CombineSpan, Id, Project, RevolveAxis, RevolveTurn};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::{amount_schema, Amount, Read};
use crate::tool::{self, Ctx, Refusal, Stage, Tool};
use crate::tools::doc::{answer, outcome};

/// What the tool does to the body of the part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Op {
    /// Add to the body; in an empty part, the first body.
    #[default]
    Join,
    Cut,
    Intersect,
}

impl Op {
    /// The kernel's code of the boolean: 1 adds, 0 cuts, 2 keeps the common part.
    fn occt(self) -> u8 {
        match self {
            Op::Join => 1,
            Op::Cut => 0,
            Op::Intersect => 2,
        }
    }
}

/// Which way the tool grows from the sketch plane.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Direction {
    Forward,
    Backward,
    /// Half each way.
    Both,
}

impl Direction {
    fn reach(self) -> Reach {
        match self {
            Direction::Forward => Reach::Forward,
            Direction::Backward => Reach::Backward,
            Direction::Both => Reach::BothWays,
        }
    }
}

/// How far an extrusion goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Reaches {
    /// By `distance`.
    #[default]
    Distance,
    /// The whole way through the body, whatever `distance` says.
    Through,
}

/// THE SKETCH A FEATURE IS MADE FROM, and the closed contours it takes: those named, or the one closed contour
/// there is. Of several, none is chosen for the model, as none is chosen for a person: two squares apart taken at once
/// make a part of two pieces.
struct Source {
    sketch: Id,
    contours: Vec<Id>,
}

fn source(project: &Project, sketch: Id, named: &Option<Vec<Id>>) -> Result<Source, Refusal> {
    let si = project.sketch_index(sketch).ok_or_else(|| Refusal::new("no-sketch", &format!("There is no sketch {sketch}."), Stage::Validate))?;
    let closed: Vec<Id> = project.sketches[si].contour_ids.iter().copied().filter(|c| project.contour_profile_xy(*c).is_some()).collect();
    let contours = match named {
        Some(list) => {
            let open: Vec<&Id> = list.iter().filter(|c| !closed.contains(c)).collect();
            if !open.is_empty() || list.is_empty() {
                return Err(Refusal::new("no-contour", &format!("Contours {open:?} are not closed contours of sketch {sketch}; the closed ones are {closed:?}."), Stage::Validate));
            }
            list.clone()
        }
        None => match closed.as_slice() {
            [one] => vec![*one],
            [] => {
                return Err(Refusal::new("no-contour", &format!("Sketch {sketch} has no closed contour."), Stage::Validate).with_hint("Read sketch_info: an open contour or a conflict keeps it open."))
            }
            many => {
                return Err(Refusal::new("several-contours", &format!("Sketch {sketch} has {} closed contours: {many:?}.", many.len()), Stage::Validate).with_hint("Name the ones to take in contours."))
            }
        },
    };
    Ok(Source { sketch, contours })
}

/// The body of the part that owns the sketch, which the feature joins, cuts or meets; none in an empty part.
fn owner_body(project: &Project, sketch: Id) -> Option<Id> {
    project.sketch_owner(sketch).and_then(|o| project.active_body(o))
}

/// A cut or a meeting needs a body to act on.
fn needs_body(op: Op, body: Option<Id>, sketch: Id) -> Result<(), Refusal> {
    if op != Op::Join && body.is_none() {
        return Err(
            Refusal::new("no-body", &format!("The part of sketch {sketch} has no body to {}.", if op == Op::Cut { "cut" } else { "meet" }), Stage::Validate).with_hint("Join a first body, then cut.")
        );
    }
    Ok(())
}

/// A size that must be more than nothing.
fn positive(project: &Project, a: &Amount, field: &str) -> Result<Read, Refusal> {
    let r = a.read(project, field)?;
    if r.value > 0.0 {
        Ok(r)
    } else {
        Err(Refusal::new("bad-size", &format!("{field} is {}; it must be more than 0.", r.value), Stage::Validate))
    }
}

/// The node laid and the body the part stands as after it.
struct Laid {
    node: Id,
    body: Id,
}

fn laid_answer(ctx: &Ctx, laid: Laid) -> tool::Answer {
    let mut a = answer("feature", json!(laid.node));
    a.insert("body".into(), json!(laid.body));
    outcome(ctx, a)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtrudeArgs {
    sketch: Id,
    contours: Option<Vec<Id>>,
    distance: Option<Amount>,
    #[serde(default)]
    op: Op,
    direction: Option<Direction>,
    second: Option<Amount>,
    #[serde(default)]
    reaches: Reaches,
}

pub const EXTRUDE: Tool = Tool {
    name: "extrude",
    description: "Extrude closed contours of a sketch along its normal: op \"join\" (the default; in an empty part the first body), \"cut\" or \"intersect\" the part's body. distance in mm or an expression; reaches \"through\" cuts the whole way instead. direction \"forward\" (along the normal), \"backward\" or \"both\" (half each way); by default a cut from a sketch on a face goes backward, into the body, everything else forward. second adds a length on the other side. contours names the closed contours to take; without it the sketch's one closed contour is taken.",
    schema: || {
        json!({ "type": "object", "properties": {
            "sketch": { "type": "integer" },
            "contours": { "type": "array", "items": { "type": "integer" }, "description": "Keys of closed contours (sketch_info); without it the one closed contour." },
            "distance": amount_schema("Length along the normal, mm"),
            "op": { "type": "string", "enum": ["join", "cut", "intersect"], "default": "join" },
            "direction": { "type": "string", "enum": ["forward", "backward", "both"] },
            "second": amount_schema("Length on the other side, mm"),
            "reaches": { "type": "string", "enum": ["distance", "through"], "default": "distance" },
        }, "required": ["sketch"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ExtrudeArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = source(project, a.sketch, &a.contours)?;
        let body = owner_body(project, a.sketch);
        needs_body(a.op, body, a.sketch)?;
        let through = a.reaches == Reaches::Through;
        if through && body.is_none() {
            return Err(Refusal::new("no-body", "Nothing stands to go through: the part has no body.", Stage::Validate));
        }
        let height = match (&a.distance, through) {
            (Some(d), _) => positive(project, d, "distance")?,
            (None, true) => Read { value: 1.0, expr: None },
            (None, false) => return Err(Refusal::new("arguments", "distance is wanted unless reaches is \"through\".", Stage::Validate)),
        };
        let down = a.second.as_ref().map(|s| positive(project, s, "second")).transpose()?;
        if down.is_some() && a.direction == Some(Direction::Both) {
            return Err(Refusal::new("arguments", "second cannot go with direction \"both\": half of distance already goes each way.", Stage::Validate));
        }
        // the window's default: a cut from a sketch on a face goes into the body (`smart_flip`)
        let plane = project.sketch_index(a.sketch).map(|si| project.sketches[si].plane).unwrap_or_default();
        let into = a.op == Op::Cut && qymcad_doc::ops::cut_goes_into_the_body(&plane);
        let reach = a.direction.map_or(if into { Reach::Backward } else { Reach::Forward }, Direction::reach);
        let down_value = down.as_ref().map_or(0.0, |d| d.value);
        let laid = ctx
            .doc
            .edit("f-extrusion", |p| {
                if let Some(owner) = p.sketch_owner(src.sketch) {
                    p.set_active_component(Some(owner));
                }
                let node = match body {
                    None => p.add_extrude_multi(src.sketch, src.contours.clone(), height.value, reach, down_value, Vec::new()),
                    Some(b) => p.add_combine_multi_op(b, src.sketch, src.contours.clone(), CombineSpan { height: height.value, down: down_value, extent: Extent { through, reach }, fill: &[] }, a.op.occt()),
                };
                if node == 0 {
                    return Err("cmd-op-failed".to_string());
                }
                p.set_feat_dim(node, "height", height.expr.clone().unwrap_or_default());
                p.set_feat_dim(node, "down", down.as_ref().and_then(|d| d.expr.clone()).unwrap_or_default());
                let body = if body.is_none() { p.finish_base_body(node, 1) } else { node };
                Ok(Laid { node, body })
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid_answer(ctx, laid))
    },
};

/// The axis a revolution turns about.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum AxisArg {
    /// "x" or "y" of the sketch.
    Base(BaseAxis),
    /// A line of the sketch by its key, or a datum axis.
    Line {
        line: Id,
    },
    Datum {
        datum: Id,
    },
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BaseAxis {
    X,
    Y,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevolveArgs {
    sketch: Id,
    contours: Option<Vec<Id>>,
    axis: AxisArg,
    angle: Option<Amount>,
    #[serde(default)]
    op: Op,
    direction: Option<Direction>,
}

pub const REVOLVE: Tool = Tool {
    name: "revolve",
    description: "Turn closed contours of a sketch about an axis: \"x\" or \"y\" of the sketch, {\"line\": key} a line of the sketch (sketch_info), or {\"datum\": key} a datum axis. angle in degrees or an expression, 360 by default; direction \"forward\", \"backward\" or \"both\" (half each way). op \"join\" (the default), \"cut\" or \"intersect\" the part's body. The contour must lie to one side of the axis.",
    schema: || {
        json!({ "type": "object", "properties": {
            "sketch": { "type": "integer" },
            "contours": { "type": "array", "items": { "type": "integer" } },
            "axis": { "description": "\"x\", \"y\", {\"line\": key} or {\"datum\": key}.", "oneOf": [
                { "type": "string", "enum": ["x", "y"] },
                { "type": "object", "properties": { "line": { "type": "integer" } }, "required": ["line"], "additionalProperties": false },
                { "type": "object", "properties": { "datum": { "type": "integer" } }, "required": ["datum"], "additionalProperties": false },
            ] },
            "angle": amount_schema("Degrees, 360 by default"),
            "op": { "type": "string", "enum": ["join", "cut", "intersect"], "default": "join" },
            "direction": { "type": "string", "enum": ["forward", "backward", "both"] },
        }, "required": ["sketch", "axis"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: RevolveArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = source(project, a.sketch, &a.contours)?;
        let body = owner_body(project, a.sketch);
        needs_body(a.op, body, a.sketch)?;
        let angle = match &a.angle {
            Some(v) => positive(project, v, "angle")?,
            None => Read { value: 360.0, expr: None },
        };
        if angle.value > 360.0 {
            return Err(Refusal::new("bad-size", &format!("angle is {}; a turn is 360 at most.", angle.value), Stage::Validate));
        }
        let axis = match a.axis {
            AxisArg::Base(BaseAxis::X) => RevolveAxis { axis: 0, datum: 0, line: 0 },
            AxisArg::Base(BaseAxis::Y) => RevolveAxis { axis: 1, datum: 0, line: 0 },
            AxisArg::Line { line } => {
                let si = project.sketch_index(a.sketch).unwrap_or_default();
                let is_line = project.sketches[si].entities.iter().any(|e| e.id == line && matches!(e.kind, qymcad_core::model::EntityKind::Line { .. }));
                if !is_line {
                    return Err(Refusal::new("wrong-kind", &format!("{line} is no line of sketch {}.", a.sketch), Stage::Validate));
                }
                RevolveAxis { axis: 0, datum: 0, line }
            }
            AxisArg::Datum { datum } => {
                if !project.datum_axes.iter().any(|d| d.id == datum) {
                    return Err(Refusal::new("no-datum", &format!("There is no datum axis {datum}."), Stage::Validate));
                }
                RevolveAxis { axis: 0, datum, line: 0 }
            }
        };
        let reach = a.direction.map_or(Reach::Forward, Direction::reach);
        let laid = ctx
            .doc
            .edit("f-revolution", |p| {
                if let Some(owner) = p.sketch_owner(src.sketch) {
                    p.set_active_component(Some(owner));
                }
                // with no body there is nothing to boolean against: the turn is a new body joined into the part
                let with = body.unwrap_or(0);
                let op = if body.is_some() { a.op.occt() } else { 1 };
                let node = p.add_revolve_multi_op(src.sketch, src.contours.clone(), axis, RevolveTurn { angle: angle.value, reach }, with, op);
                if node == 0 {
                    return Err("cmd-op-failed".to_string());
                }
                p.set_feat_dim(node, "angle", angle.expr.clone().unwrap_or_default());
                let body = if with == 0 { p.finish_base_body(node, 1) } else { node };
                Ok(Laid { node, body })
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid_answer(ctx, laid))
    },
};

/// THE BODY A FINISHING TOOL WORKS ON: the one named, or the one body of the active part.
fn target(project: &Project, body: Option<crate::args::BodyRef>) -> Result<Id, Refusal> {
    match body {
        Some(b) => b.resolve(project),
        None => crate::args::BodyRef::Part(project.current_ctx()).resolve(project).map_err(|r| r.with_hint("Name the body: {\"body\": key} or {\"part\": key}.")),
    }
}

/// The schema of the body a finishing tool takes; without it, the active part's one body.
fn body_field() -> Value {
    let mut s = crate::args::body_schema();
    s["description"] = json!("{\"body\": key} or {\"part\": key}; without it, the one body of the active part.");
    s
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilletArgs {
    body: Option<crate::args::BodyRef>,
    edges: crate::args::QueryArg,
    radius: Amount,
}

pub const FILLET: Tool = Tool {
    name: "fillet",
    description: "Round edges of a body to a radius (mm or an expression). edges is a query kept with the feature and found again on every rebuild: {\"adjacent\": \"top\"} rounds every edge of the top face, {\"between\": {\"one\": \"top\", \"other\": \"front\"}} the edge where they meet, {\"ids\": [...]} edges by key (list_edges). A query finding no edge is refused before anything is laid.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "edges": crate::args::query_schema(),
            "radius": amount_schema("Radius, mm"),
        }, "required": ["edges", "radius"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: FilletArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let r = positive(project, &a.radius, "radius")?;
        let edges = crate::args::reference(&a.edges, crate::args::Expect::Some);
        let _ = crate::args::resolve(project, body, &edges, crate::args::Element::Edges)?;
        let laid = ctx
            .doc
            .edit("f-fillet", |p| {
                let node = p.add_fillet_ref(body, r.value, edges);
                p.set_feat_dim(node, "radius", r.expr.clone().unwrap_or_default());
                Ok(Laid { node, body: node })
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid_answer(ctx, laid))
    },
};

/// The second size of an asymmetric chamfer: a second leg, or the angle from the first.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Second {
    Distance(Amount),
    Angle(Amount),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChamferArgs {
    body: Option<crate::args::BodyRef>,
    edges: crate::args::QueryArg,
    distance: Amount,
    second: Option<Second>,
}

pub const CHAMFER: Tool = Tool {
    name: "chamfer",
    description: "Cut edges of a body at a slant: distance is the leg (mm or an expression). second {\"distance\": d} makes the other leg d, {\"angle\": deg} sets the angle from the first leg; without it both legs are equal. edges is a query, as for fillet; a symmetric chamfer keeps the query and finds it again on every rebuild, an asymmetric one keeps the edges found now.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "edges": crate::args::query_schema(),
            "distance": amount_schema("Leg, mm"),
            "second": { "type": "object", "description": "{\"distance\": mm} or {\"angle\": degrees}.", "minProperties": 1, "maxProperties": 1 },
        }, "required": ["edges", "distance"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ChamferArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let d = positive(project, &a.distance, "distance")?;
        let edges = crate::args::reference(&a.edges, crate::args::Expect::Some);
        let found = crate::args::resolve(project, body, &edges, crate::args::Element::Edges)?;
        let second = match &a.second {
            None => None,
            Some(Second::Distance(v)) => Some(Asymmetric { mode: qymcad_core::feature::ChamferMode::TwoDist, size: positive(project, v, "second.distance")? }),
            Some(Second::Angle(v)) => {
                let r = positive(project, v, "second.angle")?;
                if r.value >= 90.0 {
                    return Err(Refusal::new("bad-size", &format!("second.angle is {}; below 90 is taken.", r.value), Stage::Validate));
                }
                Some(Asymmetric { mode: qymcad_core::feature::ChamferMode::DistAngle, size: r })
            }
        };
        let laid = ctx
            .doc
            .edit("f-chamfer", |p| {
                let node = match &second {
                    None => p.add_chamfer_ref(body, d.value, edges),
                    // the asymmetric shapes measure from a side of each edge, so they stand on the edges found now, as
                    // the window lays them from picked edges
                    Some(asym) => p.add_chamfer_ex(body, d.value, qymcad_core::model::ChamferShape { mode: asym.mode, d2: asym.size.value, flip: false, ref_face: 0 }, found.clone()),
                };
                p.set_feat_dim(node, "dist", d.expr.clone().unwrap_or_default());
                p.set_feat_dim(node, "d2", second.as_ref().and_then(|asym| asym.size.expr.clone()).unwrap_or_default());
                Ok(Laid { node, body: node })
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid_answer(ctx, laid))
    },
};

/// The kind of a hole.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HoleKind {
    #[default]
    Plain,
    Counterbore,
    Countersink,
}

impl HoleKind {
    /// The kernel's code: 0 plain, 1 counterbore, 2 countersink.
    fn code(self) -> u8 {
        match self {
            HoleKind::Plain => 0,
            HoleKind::Counterbore => 1,
            HoleKind::Countersink => 2,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HoleArgs {
    body: Option<crate::args::BodyRef>,
    face: Option<crate::args::QueryArg>,
    offset: Option<[f64; 2]>,
    at: Option<[f64; 3]>,
    sketch: Option<Id>,
    diameter: Amount,
    depth: Amount,
    #[serde(default)]
    kind: HoleKind,
    recess_diameter: Option<Amount>,
    recess_depth: Option<Amount>,
    #[serde(default)]
    direction: Drill,
}

/// WHICH WAY THE HOLES OF A SKETCH GO: against the sketch's normal - into the body under a sketch on its face - or
/// along it, for a sketch standing below the body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Drill {
    #[default]
    AgainstNormal,
    AlongNormal,
}

pub const HOLE: Tool = Tool {
    name: "hole",
    description: "Drill a hole square into a flat face: face is a query finding one face; the hole stands at its centre, offset [u, v] mm along the face's own axes, or at a point at [x, y, z] on the face. Or sketch: a hole at every lone point of that sketch (draw them with {\"point\": ...}), drilled against the sketch normal, or along it with direction \"along_normal\" (a sketch below the body). diameter and depth in mm or expressions; a depth past the body goes through. kind \"plain\", \"counterbore\" (a flat recess) or \"countersink\" (a cone), the recess recess_diameter across and recess_depth deep.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "face": crate::args::query_schema(),
            "offset": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2, "description": "From the face centre along its axes, mm." },
            "at": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "A point on the face, mm." },
            "sketch": { "type": "integer", "description": "Drill at the lone points of this sketch instead of a face." },
            "diameter": amount_schema("Diameter, mm"),
            "depth": amount_schema("Depth, mm"),
            "kind": { "type": "string", "enum": ["plain", "counterbore", "countersink"], "default": "plain" },
            "recess_diameter": amount_schema("Recess diameter, mm"),
            "recess_depth": amount_schema("Recess depth, mm"),
            "direction": { "type": "string", "enum": ["against_normal", "along_normal"], "default": "against_normal", "description": "For sketch holes: against the sketch normal (into a body under a sketch on its face), or along it." },
        }, "required": ["diameter", "depth"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: HoleArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let dia = positive(project, &a.diameter, "diameter")?;
        let depth = positive(project, &a.depth, "depth")?;
        let recess = match a.kind {
            HoleKind::Plain => None,
            _ => {
                let missing = || Refusal::new("arguments", "A counterbore or a countersink needs recess_diameter and recess_depth.", Stage::Validate);
                let rd = positive(project, a.recess_diameter.as_ref().ok_or_else(missing)?, "recess_diameter")?;
                let rh = positive(project, a.recess_depth.as_ref().ok_or_else(missing)?, "recess_depth")?;
                if rd.value <= dia.value {
                    return Err(Refusal::new("bad-size", &format!("recess_diameter {} is not wider than the hole {}.", rd.value, dia.value), Stage::Validate));
                }
                Some(Recess { diameter: rd, depth: rh })
            }
        };
        let tool = qymcad_core::model::HoleTool {
            kind: a.kind.code(),
            diameter: dia.value,
            depth: depth.value,
            dia2: recess.as_ref().map_or(0.0, |r| r.diameter.value),
            depth2: recess.as_ref().map_or(0.0, |r| r.depth.value),
        };
        let place = match (&a.face, a.sketch) {
            (Some(_), Some(_)) | (None, None) => return Err(Refusal::new("arguments", "Give a face or a sketch, one of the two.", Stage::Validate)),
            (None, Some(sketch)) => {
                if project.sketch_index(sketch).is_none() {
                    return Err(Refusal::new("no-sketch", &format!("There is no sketch {sketch}."), Stage::Validate));
                }
                if project.sketch_isolated_points(sketch).is_empty() {
                    return Err(Refusal::new("no-points", &format!("Sketch {sketch} has no lone points to drill at."), Stage::Validate).with_hint("Draw them with {\"point\": {\"at\": [x, y]}}."));
                }
                Place::Sketch(sketch)
            }
            (Some(face), None) => {
                let key = crate::args::face_key(project, body, face)?;
                let frame = qymcad_doc::ops::FaceFrame { centre: key.centroid, normal: key.normal };
                let point = match (a.at, a.offset) {
                    (Some(_), Some(_)) => return Err(Refusal::new("arguments", "Give offset or at, not both.", Stage::Validate)),
                    (Some(at), None) => at,
                    (None, off) => {
                        let [u, v] = off.unwrap_or([0.0, 0.0]);
                        frame.point_at(qymcad_doc::ops::FaceOffset { u, v })
                    }
                };
                Place::Face { key, point }
            }
        };
        let laid = ctx
            .doc
            .edit("f-operation", |p| {
                let node = match place {
                    Place::Face { key, point } => p.add_hole_at(body, key, point, tool),
                    Place::Sketch(sketch) => p.add_hole_from_sketch(body, sketch, tool, a.direction == Drill::AlongNormal),
                };
                p.set_feat_dim(node, "diameter", dia.expr.clone().unwrap_or_default());
                p.set_feat_dim(node, "depth", depth.expr.clone().unwrap_or_default());
                p.set_feat_dim(node, "dia2", recess.as_ref().and_then(|r| r.diameter.expr.clone()).unwrap_or_default());
                p.set_feat_dim(node, "depth2", recess.as_ref().and_then(|r| r.depth.expr.clone()).unwrap_or_default());
                Ok(Laid { node, body: node })
            })
            .map_err(tool::doc_refusal)?;
        let air = qymcad_doc::ops::holes_in_air(ctx.doc.project(), laid.node);
        let mut out = laid_answer(ctx, laid);
        if !air.is_empty() {
            out.insert("warnings".into(), json!(air.iter().map(|at| hole_in_air(ctx, body, *at)).collect::<Vec<_>>()));
        }
        Ok(out)
    },
};

/// The warning of a hole that cut nothing: where it stands and where the body it was to drill is, both in the part's
/// frame - the usual cause is a point laid as if the body began at the origin.
fn hole_in_air(ctx: &Ctx, body: qymcad_core::model::Id, at: [f64; 3]) -> Value {
    let span = ctx.doc.project().mesh_index(body).and_then(|mi| ctx.doc.project().bodies[mi].mesh.bounds());
    let message = match span {
        Some(b) => format!("The hole at {at:?} meets no material: the body spans x {}..{}, y {}..{}, z {}..{}.", b.min.x, b.max.x, b.min.y, b.max.y, b.min.z, b.max.z),
        None => format!("The hole at {at:?} meets no material."),
    };
    json!({ "code": "hole-in-air", "at": at, "message": message })
}

/// The recess over a counterbored or countersunk hole.
struct Recess {
    diameter: Read,
    depth: Read,
}

/// The shape of an asymmetric chamfer and its second size.
struct Asymmetric {
    mode: qymcad_core::feature::ChamferMode,
    size: Read,
}

/// Where a hole is drilled.
#[derive(Clone, Copy)]
enum Place {
    Face { key: qymcad_core::feature::FaceKey, point: [f64; 3] },
    Sketch(Id),
}

/// Which way the wall of a shell goes from the surface.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Side {
    #[default]
    Inward,
    Outward,
    Centred,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShellArgs {
    body: Option<crate::args::BodyRef>,
    open: crate::args::QueryArg,
    thickness: Amount,
    #[serde(default)]
    side: Side,
}

pub const SHELL: Tool = Tool {
    name: "shell",
    description: "Hollow a body to a wall of thickness (mm or an expression), leaving open the faces the query open finds (\"top\" for a box open at the top). side \"inward\" (the default: the outside stays), \"outward\" (the inside stays) or \"centred\".",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "open": crate::args::query_schema(),
            "thickness": amount_schema("Wall, mm"),
            "side": { "type": "string", "enum": ["inward", "outward", "centred"], "default": "inward" },
        }, "required": ["open", "thickness"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ShellArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let body = target(project, a.body)?;
        let t = positive(project, &a.thickness, "thickness")?;
        let faces = crate::args::resolve(project, body, &crate::args::reference(&a.open, crate::args::Expect::Some), crate::args::Element::Faces)?;
        let side = match a.side {
            Side::Inward => qymcad_core::feature::ShellSide::Inward,
            Side::Outward => qymcad_core::feature::ShellSide::Outward,
            Side::Centred => qymcad_core::feature::ShellSide::Centred,
        };
        let laid = ctx
            .doc
            .edit("f-shell", |p| {
                let node = p.add_shell_mode(body, t.value, faces.clone(), side);
                p.set_feat_dim(node, "thickness", t.expr.clone().unwrap_or_default());
                Ok(Laid { node, body: node })
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid_answer(ctx, laid))
    },
};
