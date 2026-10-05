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
