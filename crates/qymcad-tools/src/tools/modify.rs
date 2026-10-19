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

pub(crate) fn positive(project: &Project, a: &Amount, field: &str) -> Result<Read, Refusal> {
    let r = a.read(project, field)?;
    if r.value > 0.0 {
        Ok(r)
    } else {
        Err(Refusal::new("bad-size", &format!("{field} is {}; it must be more than 0.", r.value), Stage::Validate))
    }
}

/// The answer of a node that leaves one body.
fn laid(ctx: &Ctx, node: Id) -> Answer {
    laid_node(ctx, node)
}

/// What a node laid: the feature and the body it made, with the outcome of the document.
pub(crate) fn laid_node(ctx: &Ctx, node: Id) -> Answer {
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
                        p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key }, &if on { expr.clone() } else { String::new() }, qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
                    }
                    p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key: STEPS[i] }, &expr, qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
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
                p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key: "angle" }, &angle.expr.clone().unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
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
        let pieces = ctx.doc.shape(src).and_then(|s| qymcad_kernel::ask::split_pieces(s, qymcad_core::feature::PlaneAt { origin, normal: n })).unwrap_or(0);
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
                p.write_feat_formula(&qymcad_core::model::FeatNumber { node: first, key: "offset" }, &offset.expr.clone().unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
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
                p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key: "dist" }, &d.expr.clone().unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
                Ok(node)
            })
            .map_err(tool::doc_refusal)?;
        Ok(laid(ctx, node))
    },
};

/// Which face a size between faces moves, as a caller names it.
#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum MovesArg {
    #[default]
    First,
    Second,
    Both,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FaceGapArgs {
    body: Option<BodyRef>,
    face: QueryArg,
    other: Option<QueryArg>,
    hole: Option<[f64; 3]>,
    size: Amount,
    #[serde(default)]
    moves: MovesArg,
}

pub const FACE_GAP: Tool = Tool {
    name: "face_gap",
    description: "Set the distance between two parallel flat faces of a body - the thickness of a plate, the width of a block - where the body has no sketch to change it in (a STEP, a mesh made a body). face and other are queries finding one face each; size is the distance wanted, mm (a number or an expression). moves says which face moves: first (face, by default), second (other) or both by half, the middle standing still. One node; its size stays editable. Roundings and chamfers made exact beside a moved face keep their size. With hole - a point on the wall of a hole - in place of other, size is the distance from the hole's axis to face, which must run along the axis, and the hole moves there: its size, depth and entry face as they were.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "face": args::query_schema(),
            "other": args::query_schema(),
            "hole": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "A point on the wall of a hole, mm, in the part's frame: the hole is moved instead of a face." },
            "size": amount_schema("The distance between the faces, mm"),
            "moves": { "type": "string", "enum": ["first", "second", "both"], "description": "Which face moves; first by default." },
        }, "required": ["face", "size"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        use qymcad_core::feature::GapMoves;
        let a: FaceGapArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        let one = args::face_key(project, src, &a.face)?;
        let size = positive(project, &a.size, "size")?;
        if let Some(at) = a.hole {
            return move_hole(ctx, src, one, at, size);
        }
        let other = args::face_key(project, src, a.other.as_ref().ok_or_else(|| Refusal::new("missing", "Give other - the second face - or hole - a point on the wall of a hole.", Stage::Validate))?)?;
        let n = one.normal;
        let turn = n[0] * other.normal[0] + n[1] * other.normal[1] + n[2] * other.normal[2];
        let apart = ((0..3).map(|i| n[i] * (other.centroid[i] - one.centroid[i])).sum::<f64>()).abs();
        if turn.abs() < 1.0 - 1e-6 || apart < 1e-3 {
            return Err(Refusal::new("not-parallel", "The two faces are not parallel planes standing apart.", Stage::Validate).with_hint("Give two flat faces facing each other or turned the same way."));
        }
        if (apart - size.value).abs() < 1e-9 && size.expr.is_none() {
            return Err(Refusal::new("bad-size", &format!("The faces already stand {apart} apart."), Stage::Validate));
        }
        let moves = match a.moves {
            MovesArg::First => GapMoves::One,
            MovesArg::Second => GapMoves::Other,
            MovesArg::Both => GapMoves::Both,
        };
        let node = ctx
            .doc
            .edit("f-face-gap", |p| {
                let part = p.body_owner(src);
                p.set_active_component(part);
                let node = p.add_face_gap(src, qymcad_core::model::Gap { one, other, size: size.value, moves });
                p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key: "gap" }, &size.expr.clone().unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
                Ok(node)
            })
            .map_err(tool::doc_refusal)?;
        let mut out = laid(ctx, node);
        out.insert("was".into(), json!(apart));
        Ok(out)
    },
};

/// What a sketch taken off a face goes on to do, as a caller names it.
#[derive(Deserialize, Default, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
enum FromFaceArg {
    /// Lay the sketch alone.
    #[default]
    Sketch,
    /// Lay the sketch and redraw the boss or pocket on the face by it.
    Replace,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SketchFromFaceArgs {
    body: Option<BodyRef>,
    face: QueryArg,
    #[serde(default)]
    action: FromFaceArg,
    height: Option<Amount>,
}

/// What one call laid: the sketch, and the node that redrew the boss or pocket by it.
struct FromFaceLaid {
    sketch: qymcad_core::model::FaceSketch,
    node: Option<Id>,
}

pub const SKETCH_FROM_FACE: Tool = Tool {
    name: "sketch_from_face",
    description: "Take a sketch off a flat face of a body - a STEP has no sketch behind its bosses. The outline of the face is copied into a sketch on it as curves of its own, given the relations it shows (horizontal, vertical, parallel, perpendicular, tangent, concentric, equal) and sizes until it is fully defined, placed from the outline of the face the boss stands on, sizes rounded to a tenth. face is a query finding one flat face: the top of a boss or the floor of a pocket. action replace also takes the boss or pocket off and lays it again by the sketch, one node, its height from its base (height, mm, a number or an expression; as found by default): from then on the sizes of the sketch drive the body. A rim rounded or cut all round, at the face or at the base, goes with the boss and is laid again its size (rim_at_face, rim_at_base in the answer; editable as rim and base_rim). A boss whose walls lean, whose rim is rounded only in part or to different sizes, whose walls end at different heights or with a hole going deeper is refused for replace; sketch still copies the outline. The answer says what was found and what the sketch holds: constraints, sizes, freedoms left, sizes rounded.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "face": args::query_schema(),
            "action": { "type": "string", "enum": ["sketch", "replace"], "description": "sketch: the sketch alone (by default); replace: the boss or pocket redrawn by it." },
            "height": amount_schema("The height of the boss or the depth of the pocket redrawn, mm; as found by default"),
        }, "required": ["face"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SketchFromFaceArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        let key = args::face_key(project, src, &a.face)?;
        let height = a.height.as_ref().map(|h| positive(project, h, "height")).transpose()?;
        let name = qymcad_i18n::tr1("g-sketch-n", "n", &(project.sketches.len() + 1).to_string());
        let found = ctx.doc.with_kernel(|p, k| p.prism_at(src, key.id, k));
        let what = match &found {
            Ok(p) => {
                let rim = |r: Option<qymcad_core::model::RimBlend>| r.map(|r| json!({ "cut": if r.cut == qymcad_core::model::RimCut::Round { "round" } else { "chamfer" }, "size": r.size }));
                json!({ "kind": if p.kind == qymcad_core::model::PrismKind::Boss { "boss" } else { "pocket" }, "height": p.height, "walls": p.walls.len(), "holes": p.inner.len(), "rim_at_face": rim(p.rims.at_face), "rim_at_base": rim(p.rims.at_base) })
            }
            Err(e) => json!({ "kind": "none", "why": e.to_string() }),
        };
        let replace = a.action == FromFaceArg::Replace;
        let prism = match found {
            Ok(p) => Some(p),
            Err(e) if replace => return Err(Refusal::new("not-a-prism", &format!("Nothing to redraw on that face: {e}."), Stage::Validate).with_hint("Give action sketch to copy the outline only.")),
            Err(_) => None,
        };
        let outline = prism.as_ref().map(|p| p.outline.clone()).unwrap_or_default();
        let profile = prism.as_ref().map(|p| p.profile.clone()).unwrap_or_default();
        let redraw = prism.filter(|_| replace);
        let laid = ctx
            .doc
            .edit_with_kernel("f-sketch-from-face", |p, k| {
                let part = p.body_owner(src);
                p.set_active_component(part);
                let sketch = p.add_sketch_from_face(qymcad_core::model::FromFace { body: src, face: key, profile: &profile, outline: &outline, name }, k).ok_or("msg-sketch-from-face-no-outline")?;
                let node = redraw.as_ref().map(|prism| p.add_redrawn(src, prism, sketch.sketch));
                // the height asked, a number or an expression kept on the node
                for (node, h) in node.iter().zip(height.as_ref()) {
                    if let Some(qymcad_core::feature::FeatureKind::Redrawn { height: stored, .. }) = p.timeline.iter_mut().find(|n| n.id == *node).map(|n| &mut n.kind) {
                        *stored = h.value;
                    }
                    p.write_feat_formula(&qymcad_core::model::FeatNumber { node: *node, key: "height" }, &h.expr.clone().unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
                }
                Ok(FromFaceLaid { sketch, node })
            })
            .map_err(tool::doc_refusal)?;
        let d = &laid.sketch.defined;
        let held = json!({
            "id": laid.sketch.sketch,
            "constraints": d.relations,
            "sizes": d.sizes,
            "free": d.free,
            "rounded": d.rounded.iter().map(|r| json!({ "was": r.was, "now": r.now })).collect::<Vec<_>>(),
        });
        let mut out = match laid.node {
            Some(node) => laid_node(ctx, node),
            None => outcome(ctx, answer("sketch", json!(laid.sketch.sketch))),
        };
        out.insert("found".into(), what);
        out.insert("sketch".into(), held);
        Ok(out)
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecogniseFeatureArgs {
    body: Option<BodyRef>,
    at: [f64; 3],
    size: Option<Amount>,
    depth: Option<Amount>,
    tolerance: Option<f64>,
}

pub const RECOGNISE_FEATURE: Tool = Tool {
    name: "recognise_feature",
    description: "Make a hole, a rounding or a chamfer of a body exact and editable - on a body made of a mesh (its hole a ring of flat strips), a recognised body or a STEP. at is a point on a face of the feature: the wall of a hole, a strip of a rounding, the slanted face of a chamfer. One node takes the feature's faces off and lays it down again by its own tool, its size brought to what was drawn (a tenth of a mm; a hole to a metric clearance or tapping bore within 0.15). size overrides that size - the diameter, the radius or the leg - and depth the depth of a blind hole; tolerance multiplies the body's own (1 by default, more for a rough mesh). The answer says what was found and the size measured.",
    schema: || {
        json!({ "type": "object", "properties": {
            "body": body_field(),
            "at": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "A point on a face of the feature, mm, in the part's frame." },
            "size": amount_schema("The diameter, radius or leg, mm"),
            "depth": amount_schema("The depth of a blind hole, mm"),
            "tolerance": { "type": "number", "exclusiveMinimum": 0, "description": "A multiple of the body's own tolerance; 1 by default." },
        }, "required": ["at"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        use qymcad_core::model::Exact;
        use qymcad_meshfit::feature::{feature_at, Found, Pick, Unfound};
        let a: RecogniseFeatureArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let src = target(project, a.body)?;
        let b = project.mesh_index(src).and_then(|mi| project.bodies.get(mi)).ok_or_else(|| Refusal::new("no-body", &format!("Body {src} is not built."), Stage::Resolve))?;
        // the face under the point: the one whose triangle stands nearest it
        let near = |t: usize| {
            let [p, q, r] = b.mesh.triangle(t);
            let c = [(p.x + q.x + r.x) / 3.0, (p.y + q.y + r.y) / 3.0, (p.z + q.z + r.z) / 3.0];
            (0..3).map(|i| (c[i] - a.at[i]).powi(2)).sum::<f64>()
        };
        let face = b.faces.iter().filter_map(|f| f.triangles.iter().map(|&t| near(t as usize)).min_by(f64::total_cmp).map(|d| Nearest { id: f.id, d })).min_by(|x, y| x.d.total_cmp(&y.d)).map(|n| n.id).ok_or_else(|| Refusal::new("no-face", "The body has no faces.", Stage::Resolve))?;
        let found = feature_at(&b.mesh, &b.faces, Pick { face, at: a.at }, a.tolerance.unwrap_or(1.0)).map_err(|why| {
            let words = match why {
                Unfound::NoSurface => "No surface holds the face there within the tolerance: raise tolerance (10, then 100) for a rough mesh.",
                Unfound::NoEntry => "The round wall there has no face to drill it from: a shaft or a boss, not a hole.",
                Unfound::NotTwoSides => "The rounding there is not met by two faces along its length: give a point on another strip of it.",
                Unfound::Uneven => "The legs of the chamfer there differ: an uneven chamfer is made with chamfer and second.",
                Unfound::Other => "No hole, rounding or chamfer there: give a point on the wall of a hole, a rounding, or a slanted face between two faces.",
            };
            Refusal::new("no-feature", words, Stage::Resolve)
        })?;
        let mut remake = found.remake(&b.faces).ok_or_else(|| Refusal::new("no-face", "The feature names faces the body does not have.", Stage::Resolve))?;
        let size = a.size.as_ref().map(|v| positive(project, v, "size")).transpose()?;
        let depth = a.depth.as_ref().map(|v| positive(project, v, "depth")).transpose()?;
        match &mut remake.exact {
            Exact::Hole { diameter, depth: d, .. } => {
                *diameter = size.as_ref().map_or(*diameter, |s| s.value);
                *d = depth.as_ref().map_or(*d, |s| s.value);
            }
            Exact::Round { radius, .. } => *radius = size.as_ref().map_or(*radius, |s| s.value),
            Exact::Chamfer { leg, .. } => *leg = size.as_ref().map_or(*leg, |s| s.value),
        }
        let key = match &remake.exact {
            Exact::Hole { .. } => "diameter",
            Exact::Round { .. } => "radius",
            Exact::Chamfer { .. } => "dist",
        };
        let what = match &found {
            Found::Hole(h) => json!({ "kind": "hole", "measured": h.diameter, "depth": h.depth, "through": h.reach == qymcad_core::model::Reach::Through }),
            Found::Round(r) => json!({ "kind": "rounding", "measured": r.radius }),
            Found::Chamfer(c) => json!({ "kind": "chamfer", "measured": c.leg }),
        };
        let node = ctx
            .doc
            .edit("f-recognise-feature", |p| {
                // the node stands in the body's own part, as the window works in the part it entered
                let part = p.body_owner(src);
                p.set_active_component(part);
                let node = p.add_remade_feature(src, remake);
                p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key }, &size.as_ref().and_then(|s| s.expr.clone()).unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
                p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key: "depth" }, &depth.as_ref().and_then(|s| s.expr.clone()).unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula).map_err(|e| e.to_string())?;
                Ok(node)
            })
            .map_err(tool::doc_refusal)?;
        let mut out = laid(ctx, node);
        out.insert("found".into(), what);
        Ok(out)
    },
};

/// THE HOLE AT POINT `at` OF BODY `src` moved to stand `size` from the plane of face `face`.
fn move_hole(ctx: &mut Ctx, src: qymcad_core::model::Id, face: qymcad_core::feature::FaceKey, at: [f64; 3], size: Read) -> Result<Answer, Refusal> {
    use qymcad_core::model::Exact;
    use qymcad_meshfit::feature::{feature_at, Found, Pick};
    let project = ctx.doc.project();
    let b = project.mesh_index(src).and_then(|mi| project.bodies.get(mi)).ok_or_else(|| Refusal::new("no-body", &format!("Body {src} is not built."), Stage::Resolve))?;
    let near = |t: usize| {
        let [p, q, r] = b.mesh.triangle(t);
        let c = [(p.x + q.x + r.x) / 3.0, (p.y + q.y + r.y) / 3.0, (p.z + q.z + r.z) / 3.0];
        (0..3).map(|i| (c[i] - at[i]).powi(2)).sum::<f64>()
    };
    let wall = b
        .faces
        .iter()
        .filter_map(|f| f.triangles.iter().map(|&t| near(t as usize)).min_by(f64::total_cmp).map(|d| Nearest { id: f.id, d }))
        .min_by(|x, y| x.d.total_cmp(&y.d))
        .map(|n| n.id)
        .ok_or_else(|| Refusal::new("no-face", "The body has no faces.", Stage::Resolve))?;
    let found = feature_at(&b.mesh, &b.faces, Pick { face: wall, at }, 1.0).ok();
    let hole = match found {
        Some(f @ Found::Hole(_)) => f.remake(&b.faces).ok_or_else(|| Refusal::new("no-face", "The hole names faces the body does not have.", Stage::Resolve))?,
        _ => return Err(Refusal::new("no-hole", "No hole there: give a point on the round wall of a hole.", Stage::Resolve)),
    };
    let Exact::Hole { entry, at: axis, .. } = hole.exact else { return Err(Refusal::new("no-hole", "No hole there.", Stage::Resolve)) };
    let n = face.normal;
    if (0..3).map(|i| n[i] * entry.normal[i]).sum::<f64>().abs() > 0.01 {
        return Err(Refusal::new("not-parallel", "The axis of the hole runs into that face.", Stage::Validate).with_hint("Give a flat face lying along the axis."));
    }
    let was = (0..3).map(|i| n[i] * (axis[i] - face.centroid[i])).sum::<f64>().abs();
    if (was - size.value).abs() < 1e-9 && size.expr.is_none() {
        return Err(Refusal::new("bad-size", &format!("The axis already stands {was} from the face."), Stage::Validate));
    }
    let node = ctx
        .doc
        .edit("f-face-gap", |p| {
            let part = p.body_owner(src);
            p.set_active_component(part);
            let node = p.add_hole_moved(src, qymcad_core::model::HoleMove { hole, face, size: size.value });
            p.write_feat_formula(&qymcad_core::model::FeatNumber { node, key: "gap" }, &size.expr.clone().unwrap_or_default(), qymcad_core::model::PlainNumber::ClearsFormula)
                .map_err(|e| e.to_string())?;
            Ok(node)
        })
        .map_err(tool::doc_refusal)?;
    let mut out = laid(ctx, node);
    out.insert("was".into(), json!(was));
    Ok(out)
}

/// A face and how far its nearest triangle stands from the point given.
struct Nearest {
    id: u32,
    d: f64,
}
