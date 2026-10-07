//! CHANGING A WHOLE BODY: patterns, a mirror, a boolean of two bodies, a move, a split and a pushed face - laid as the
//! window lays them, each one node and one undo step, every size a number or an expression kept with the node.

use qymcad_core::feature::FaceKey;
use qymcad_core::model::{ArrayAxis, Id, Project};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::{self, amount_schema, Amount, BodyRef, QueryArg, Read};
use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};
use crate::tools::doc::{answer, outcome};
use crate::tools::prim::{Placement, Turn};

/// The body named, or the one body of the active part.
fn target(project: &Project, body: Option<BodyRef>) -> Result<Id, Refusal> {
    match body {
        Some(b) => b.resolve(project),
        None => BodyRef::Part(project.current_ctx()).resolve(project).map_err(|r| r.with_hint("Name the body: {\"body\": key} or {\"part\": key}.")),
    }
}

fn body_field() -> Value {
    let mut s = args::body_schema();
    s["description"] = json!("{\"body\": key} or {\"part\": key}; without it, the one body of the active part.");
    s
}

fn finite(project: &Project, a: &Amount, field: &str) -> Result<Read, Refusal> {
    a.read(project, field)
}

fn positive(project: &Project, a: &Amount, field: &str) -> Result<Read, Refusal> {
    let r = a.read(project, field)?;
    if r.value > 0.0 {
        Ok(r)
    } else {
        Err(Refusal::new("bad-size", &format!("{field} is {}; it must be more than 0.", r.value), Stage::Validate))
    }
}

/// The answer of a node that leaves one body.
fn laid(ctx: &Ctx, node: Id) -> Answer {
    let mut a = answer("feature", json!(node));
    a.insert("body".into(), json!(node));
    outcome(ctx, a)
}

/// A world axis.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Along {
    X,
    Y,
    Z,
}

impl Along {
    fn index(self) -> usize {
        match self {
            Along::X => 0,
            Along::Y => 1,
            Along::Z => 2,
        }
    }
}

/// One direction of a linear pattern: along an axis, `step` apart, `count` copies counting the body itself.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    along: Along,
    step: Amount,
    count: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinearArgs {
    body: Option<BodyRef>,
    along: Along,
    step: Amount,
    count: u32,
    second: Option<Row>,
    third: Option<Row>,
}

/// A direction read: the axis, its step and its count.
struct ReadRow {
    along: Along,
    step: Read,
    count: u32,
}

fn read_row(project: &Project, row: &Row, field: &str) -> Result<ReadRow, Refusal> {
    let step = finite(project, &row.step, &format!("{field}.step"))?;
    if step.value == 0.0 {
        return Err(Refusal::new("bad-size", &format!("{field}.step is 0: the copies would stand on each other."), Stage::Validate));
    }
    if !(1..=1000).contains(&row.count) {
        return Err(Refusal::new("bad-size", &format!("{field}.count is {}; from 1 to 1000 are taken.", row.count), Stage::Validate));
    }
    Ok(ReadRow { along: row.along, step, count: row.count })
}

pub const LINEAR_PATTERN: Tool = Tool {
    name: "linear_pattern",
    description: "Copy a body in a row along a world axis: count copies (the body itself counted), step mm apart (negative goes the other way; a number or an expression). second and third, each {along, step, count}, make a grid. Copies that overlap join into one body; copies that stand apart stay pieces of the one body of the part (pieces in the answer).",
    schema: || {
        let row = json!({ "type": "object", "properties": {
            "along": { "type": "string", "enum": ["x", "y", "z"] }, "step": amount_schema("Step, mm"), "count": { "type": "integer", "minimum": 1 },
        }, "required": ["along", "step", "count"], "additionalProperties": false });
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "along": { "type": "string", "enum": ["x", "y", "z"] },
            "step": amount_schema("Step, mm"),
            "count": { "type": "integer", "minimum": 1, "description": "Copies, the body itself counted." },
            "second": row, "third": row,
        }, "required": ["along", "step", "count"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: LinearArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        let first = read_row(project, &Row { along: a.along, step: a.step.clone(), count: a.count }, "first")?;
        let second = a.second.as_ref().map(|r| read_row(project, r, "second")).transpose()?;
        let third = a.third.as_ref().map(|r| read_row(project, r, "third")).transpose()?;
        if third.is_some() && second.is_none() {
            return Err(Refusal::new("arguments", "third needs second.", Stage::Validate));
        }
        let rows = [Some(first), second, third];
        let axis = |r: &Option<ReadRow>| match r {
            Some(r) => {
                let mut d = [0.0; 3];
                d[r.along.index()] = r.step.value;
                ArrayAxis { d, count: r.count }
            }
            None => ArrayAxis { d: [0.0; 3], count: 1 },
        };
        let axes = [axis(&rows[0]), axis(&rows[1]), axis(&rows[2])];
        let node = ctx
            .doc
            .edit("f-operation", |p| {
                let node = p.add_linear_array_grid3(src, axes);
                // the expression goes on the component the row runs along, as the window keeps it, and on the step
                // the window reopens the row with
                const KEYS: [[&str; 3]; 3] = [["dx", "dy", "dz"], ["dx2", "dy2", "dz2"], ["dx3", "dy3", "dz3"]];
                const STEPS: [&str; 3] = ["step", "step2", "step3"];
                for (i, r) in rows.iter().enumerate() {
                    let expr = r.as_ref().and_then(|r| r.step.expr.clone()).unwrap_or_default();
                    for (k, key) in KEYS[i].iter().enumerate() {
                        let on = r.as_ref().is_some_and(|r| r.along.index() == k);
                        p.set_feat_dim(node, key, if on { expr.clone() } else { String::new() });
                    }
                    p.set_feat_dim(node, STEPS[i], expr);
                }
                Ok(node)
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid(ctx, node))
    },
};

/// The axis a circular pattern turns about: world Z, or a datum axis.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(untagged)]
enum TurnAxis {
    World(ZOnly),
    Datum { datum: Id },
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ZOnly {
    Z,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CircularArgs {
    body: Option<BodyRef>,
    count: u32,
    angle: Option<Amount>,
    #[serde(default = "about_z")]
    axis: TurnAxis,
}

fn about_z() -> TurnAxis {
    TurnAxis::World(ZOnly::Z)
}

pub const CIRCULAR_PATTERN: Tool = Tool {
    name: "circular_pattern",
    description: "Copy a body round an axis: count copies (the body itself counted) spread over angle degrees (360 by default: the whole turn), about world Z or {\"datum\": key} a datum axis. Copies that stand apart stay pieces of the one body of the part.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "count": { "type": "integer", "minimum": 2 },
            "angle": amount_schema("Degrees the copies spread over, 360 by default"),
            "axis": { "description": "\"z\" (the default) or {\"datum\": key}.", "oneOf": [ { "type": "string", "enum": ["z"] }, { "type": "object", "properties": { "datum": { "type": "integer" } }, "required": ["datum"], "additionalProperties": false } ] },
        }, "required": ["count"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: CircularArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        if !(2..=1000).contains(&a.count) {
            return Err(Refusal::new("bad-size", &format!("count is {}; from 2 to 1000 are taken.", a.count), Stage::Validate));
        }
        let angle = match &a.angle {
            Some(v) => positive(project, v, "angle")?,
            None => Read { value: 360.0, expr: None },
        };
        if angle.value > 360.0 {
            return Err(Refusal::new("bad-size", &format!("angle is {}; a turn is 360 at most.", angle.value), Stage::Validate));
        }
        let axis = match a.axis {
            TurnAxis::World(ZOnly::Z) => 0,
            TurnAxis::Datum { datum } => {
                if !project.datum_axes.iter().any(|d| d.id == datum) {
                    return Err(Refusal::new("no-datum", &format!("There is no datum axis {datum}."), Stage::Validate));
                }
                datum
            }
        };
        let node = ctx
            .doc
            .edit("f-operation", |p| {
                let node = p.add_circular_array_axis(src, a.count, angle.value, axis);
                p.set_feat_dim(node, "angle", angle.expr.clone().unwrap_or_default());
                Ok(node)
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid(ctx, node))
    },
};

/// The plane a mirror or a split stands on.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum PlaneArg {
    World(World),
    Datum { datum: Id },
    Face { face: QueryArg },
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum World {
    Xy,
    Xz,
    Yz,
}

/// A plane as the core keeps it on a mirror or a split: a world plane by number, a datum, or a face of the body.
struct OpPlane {
    plane: u8,
    datum: Id,
    face: Option<FaceKey>,
}

fn op_plane(project: &Project, body: Id, plane: &PlaneArg) -> Result<OpPlane, Refusal> {
    Ok(match plane {
        PlaneArg::World(w) => OpPlane {
            plane: match w {
                World::Xy => 0,
                World::Xz => 1,
                World::Yz => 2,
            },
            datum: 0,
            face: None,
        },
        PlaneArg::Datum { datum } => {
            if !project.planes.iter().any(|pl| pl.id == *datum) {
                return Err(Refusal::new("no-datum", &format!("There is no datum plane {datum}."), Stage::Validate));
            }
            OpPlane { plane: 0, datum: *datum, face: None }
        }
        PlaneArg::Face { face } => OpPlane { plane: 0, datum: 0, face: Some(args::face_key(project, body, face)?) },
    })
}

fn plane_schema() -> Value {
    json!({ "description": "\"xy\", \"xz\", \"yz\"; {\"datum\": key}; or {\"face\": query} a flat face of the body.", "oneOf": [
        { "type": "string", "enum": ["xy", "xz", "yz"] },
        { "type": "object", "properties": { "datum": { "type": "integer" } }, "required": ["datum"], "additionalProperties": false },
        { "type": "object", "properties": { "face": args::query_schema() }, "required": ["face"], "additionalProperties": false },
    ] })
}

/// What a mirror leaves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Keep {
    /// The body and its reflection, joined.
    #[default]
    Both,
    /// The reflection alone.
    Reflection,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MirrorArgs {
    body: Option<BodyRef>,
    plane: PlaneArg,
    #[serde(default)]
    keep: Keep,
}

pub const MIRROR: Tool = Tool {
    name: "mirror",
    description: "Reflect a body in a plane: \"xy\", \"xz\", \"yz\", {\"datum\": key}, or {\"face\": query} a flat face of the body (the reflection then stands against that face). keep \"both\" (the default) joins the body and its reflection; \"reflection\" leaves the reflection alone.",
    schema: || json!({ "type": "object", "properties": { "body": body_field(), "plane": plane_schema(), "keep": { "type": "string", "enum": ["both", "reflection"], "default": "both" } }, "required": ["plane"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: MirrorArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        let at = op_plane(project, src, &a.plane)?;
        let node = ctx
            .doc
            .edit("f-mirror", |p| {
                let node = p.add_mirror(src, at.plane, a.keep == Keep::Both, at.datum);
                if let Some(key) = at.face {
                    p.set_op_face(node, src, key);
                }
                Ok(node)
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid(ctx, node))
    },
};

/// What a boolean of two bodies does.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BoolOp {
    /// The tool taken away from the target.
    Cut,
    Join,
    Intersect,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BooleanArgs {
    target: BodyRef,
    tool: BodyRef,
    op: BoolOp,
}

pub const BOOLEAN: Tool = Tool {
    name: "boolean",
    description: "Combine two bodies into one: op \"cut\" takes tool away from target, \"join\" adds them, \"intersect\" keeps what they share. Both are used up into the result. For the pieces of a split, or two parts' bodies.",
    schema: || json!({ "type": "object", "properties": { "target": args::body_schema(), "tool": args::body_schema(), "op": { "type": "string", "enum": ["cut", "join", "intersect"] } }, "required": ["target", "tool", "op"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: BooleanArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let target = a.target.resolve(project)?;
        let with = a.tool.resolve(project)?;
        if target == with {
            return Err(Refusal::new("arguments", &format!("target and tool are the same body {target}."), Stage::Validate));
        }
        let op = match a.op {
            BoolOp::Cut => 0,
            BoolOp::Join => 1,
            BoolOp::Intersect => 2,
        };
        let node = ctx.doc.edit("f-operation", |p| Ok(p.add_body_boolean(target, with, op))).map_err(tool::doc_refusal)?;
        Ok(laid(ctx, node))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveArgs {
    body: Option<BodyRef>,
    at: Option<[f64; 3]>,
    #[serde(default)]
    rotate: Vec<Turn>,
}

pub const MOVE: Tool = Tool {
    name: "move",
    description: "Move a body: rotate turns about axes through the origin, in order, then at shifts it by [x, y, z] mm. A body moved before takes the new move on top of the old one (one move node).",
    schema: || {
        let mut s = crate::tools::prim::schema(json!({ "body": body_field() }), &[]);
        s["properties"]["at"]["description"] = json!("Shift, mm.");
        s
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: MoveArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        let Some(m) = (Placement { at: a.at, rotate: a.rotate }).matrix()? else {
            return Err(Refusal::new("arguments", "Nothing to move by: give at or rotate.", Stage::Validate));
        };
        let node = ctx
            .doc
            .edit("status-move-body", |p| {
                // a body standing as a move takes the new one on top, so two moves are one node
                if p.accumulate_move(src, &m) {
                    Ok(src)
                } else {
                    Ok(p.add_move(src, m))
                }
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid(ctx, node))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitArgs {
    body: Option<BodyRef>,
    plane: PlaneArg,
    #[serde(default = "no_offset")]
    offset: Amount,
}

fn no_offset() -> Amount {
    Amount::Number(0.0)
}

pub const SPLIT_BODY: Tool = Tool {
    name: "split_body",
    description: "Cut a body in pieces by a plane (\"xy\", \"xz\", \"yz\", {\"datum\": key} or {\"face\": query}), moved offset mm along its normal - a face's normal points out of the body, so a cut inside it is negative. The pieces stay bodies of the one part; boolean joins or cuts them. A plane that cuts nothing is refused.",
    schema: || json!({ "type": "object", "properties": { "body": body_field(), "plane": plane_schema(), "offset": amount_schema("Along the plane's normal, mm") }, "required": ["plane"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SplitArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        let at = op_plane(project, src, &a.plane)?;
        let offset = finite(project, &a.offset, "offset")?;
        // the timeline makes as many bodies as the cut gives, so they are counted on the live body first, as the window
        // counts them; the plane is the core's own reading of it
        let (o, n) = project.op_plane(at.plane, at.datum, at.face.map(|k| (src, k))).ok_or_else(|| Refusal::new("ref-lost", "The plane is not found.", Stage::Resolve))?;
        let origin = [o[0] + n[0] * offset.value, o[1] + n[1] * offset.value, o[2] + n[2] * offset.value];
        let pieces = ctx.doc.shape(src).and_then(|s| s.split_by_plane(origin, n, 0)).map_or(0, |v| v.len());
        if pieces < 2 {
            return Err(Refusal::new("cuts-nothing", &format!("The plane at offset {} does not cut body {src}.", offset.value), Stage::Validate).with_hint("A face's normal points out of the body: an offset inside it is negative."));
        }
        let made = ctx
            .doc
            .edit("f-operation", |p| {
                let bodies = p.add_split_body(src, at.plane, at.datum, offset.value, pieces);
                let first = bodies[0];
                if let Some(key) = at.face {
                    p.set_op_face(first, src, key);
                }
                p.set_feat_dim(first, "offset", offset.expr.clone().unwrap_or_default());
                Ok(bodies)
            })
            .map_err(tool::doc_refusal)?;
        let mut out = answer("feature", json!(made[0]));
        out.insert("pieces".into(), json!(made));
        Ok(outcome(ctx, out))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PushArgs {
    body: Option<BodyRef>,
    face: QueryArg,
    distance: Amount,
}

pub const PUSH_FACE: Tool = Tool {
    name: "push_face",
    description: "Move one flat face of a body along its own normal by distance mm (out for more, a minus for in; a number or an expression); the neighbouring faces follow, so a block stays a block. face is a query finding one face.",
    schema: || json!({ "type": "object", "properties": { "body": body_field(), "face": args::query_schema(), "distance": amount_schema("Along the face's normal, mm") }, "required": ["face", "distance"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: PushArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        let key = args::face_key(project, src, &a.face)?;
        let d = finite(project, &a.distance, "distance")?;
        if d.value == 0.0 {
            return Err(Refusal::new("bad-size", "distance is 0: the face would stay where it is.", Stage::Validate));
        }
        let node = ctx
            .doc
            .edit("f-push-face", |p| {
                let node = p.add_push_face(src, key, d.value);
                p.set_feat_dim(node, "dist", d.expr.clone().unwrap_or_default());
                Ok(node)
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid(ctx, node))
    },
};
