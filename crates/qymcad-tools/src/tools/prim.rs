//! THE PRIMITIVES: a box, a cylinder, a sphere, a cone, a torus and a prism, laid as the window lays them - the node
//! with its sizes, an expression kept for each size given as one, a move into place when `at` or `rotate` asks for
//! it, and the body joined into the one body of the active part. One call is one undo step.
//!
//! Each stands where the window stands it: a box, a cylinder, a cone and a prism on XY about the origin, a sphere and
//! a torus about the origin. `rotate` turns it about the origin first, `at` then carries it.

use qymcad_core::model::{Id, Project};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::{amount_schema, Amount, Read};
use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};
use crate::tools::doc::{answer, outcome};

/// A TURN about an axis through the origin, right-handed, in degrees.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Turn {
    axis: [f64; 3],
    degrees: f64,
}

/// Where a primitive goes from where it is made: the turns in order, then the shift.
pub(crate) struct Placement {
    pub(crate) at: Option<[f64; 3]>,
    pub(crate) rotate: Vec<Turn>,
}

impl Placement {
    /// The move, or none when the primitive stays where it is made.
    pub(crate) fn matrix(&self) -> Result<Option<[f64; 12]>, Refusal> {
        let mut m = qymcad_core::feature::PLACE_IDENTITY;
        for t in &self.rotate {
            let len = t.axis.iter().map(|v| v * v).sum::<f64>().sqrt();
            if !(len > 1e-9 && len.is_finite() && t.degrees.is_finite()) {
                return Err(Refusal::new("bad-axis", &format!("A turn needs an axis of some length and a finite angle; {:?} by {} was given.", t.axis, t.degrees), Stage::Validate));
            }
            m = qymcad_core::feature::compose12(&qymcad_core::feature::rot12_axis([0.0; 3], t.axis, t.degrees), &m);
        }
        if let Some(at) = self.at {
            if at.iter().any(|v| !v.is_finite()) {
                return Err(Refusal::new("not-a-number", &format!("at {at:?} is not a point."), Stage::Validate));
            }
            for (i, v) in at.iter().enumerate() {
                m[i * 4 + 3] += v;
            }
        }
        Ok((!qymcad_core::feature::is_identity12(&m)).then_some(m))
    }
}

/// A size of the node by the key the node keeps it under (`FeatureKind::dims`).
struct Dim {
    key: &'static str,
    read: Read,
}

/// A size that must be more than nothing.
fn size(ctx: &Ctx, a: &Amount, field: &str) -> Result<Read, Refusal> {
    let r = a.read(ctx.doc.project(), field)?;
    if r.value > 0.0 {
        Ok(r)
    } else {
        Err(Refusal::new("bad-size", &format!("{field} is {}; it must be more than 0.", r.value), Stage::Validate))
    }
}

/// A size that may be nothing, as the top of a cone.
fn size_or_none(ctx: &Ctx, a: &Amount, field: &str) -> Result<Read, Refusal> {
    let r = a.read(ctx.doc.project(), field)?;
    if r.value >= 0.0 {
        Ok(r)
    } else {
        Err(Refusal::new("bad-size", &format!("{field} is {}; it must not be below 0.", r.value), Stage::Validate))
    }
}

/// What one primitive lays: its step in the history, its sizes and where it goes.
struct Laying {
    step: &'static str,
    dims: Vec<Dim>,
    place: Placement,
}

/// LAY A PRIMITIVE as one action: the node, its expressions, the move, the join into the part's body.
fn lay(ctx: &mut Ctx, laying: Laying, add: impl FnOnce(&mut Project) -> Id) -> Result<Answer, Refusal> {
    let mat = laying.place.matrix()?;
    let laid = ctx
        .doc
        .edit(laying.step, |p| {
            let node = add(p);
            for d in &laying.dims {
                p.set_feat_dim(node, d.key, d.read.expr.clone().unwrap_or_default());
            }
            let placed = mat.map_or(node, |m| p.add_move(node, m));
            Ok(Laid { node, body: p.finish_base_body(placed, 1) })
        })
        .map_err(tool::doc_refusal)?;
    let mut a = answer("feature", json!(laid.node));
    a.insert("body".into(), json!(laid.body));
    Ok(outcome(ctx, a))
}

/// The node laid and the body the part stands as after it.
struct Laid {
    node: Id,
    body: Id,
}

/// The schema of a primitive: its own sizes, then `at` and `rotate`.
pub(crate) fn schema(sizes: Value, required: &[&str]) -> Value {
    let mut props = sizes;
    props["at"] = json!({ "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "Where the origin of the primitive goes, mm." });
    props["rotate"] = json!({
        "type": "array", "description": "Turns about axes through the origin, in order, before `at`.",
        "items": { "type": "object", "properties": {
            "axis": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
            "degrees": { "type": "number" },
        }, "required": ["axis", "degrees"], "additionalProperties": false },
    });
    json!({ "type": "object", "properties": props, "required": required, "additionalProperties": false })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoxArgs {
    x: Amount,
    y: Amount,
    z: Amount,
    at: Option<[f64; 3]>,
    #[serde(default)]
    rotate: Vec<Turn>,
}

pub const BOX: Tool = Tool {
    name: "box",
    description: "Lay a box x by y by z, standing on XY centred on the origin, in the active part; it joins the part's body. Sizes are numbers (mm) or expressions over the parameters, kept with the feature. The answer gives the feature, the body and every body measured.",
    schema: || schema(json!({ "x": amount_schema("Length along X, mm"), "y": amount_schema("Width along Y, mm"), "z": amount_schema("Height along Z, mm") }), &["x", "y", "z"]),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: BoxArgs = tool::args(arguments)?;
        let x = size(ctx, &a.x, "x")?;
        let y = size(ctx, &a.y, "y")?;
        let z = size(ctx, &a.z, "z")?;
        let (dx, dy, dz) = (x.value, y.value, z.value);
        let laying = Laying { step: "cmd-box", dims: vec![Dim { key: "dx", read: x }, Dim { key: "dy", read: y }, Dim { key: "dz", read: z }], place: Placement { at: a.at, rotate: a.rotate } };
        lay(ctx, laying, |p| p.add_box(dx, dy, dz))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CylinderArgs {
    radius: Amount,
    height: Amount,
    at: Option<[f64; 3]>,
    #[serde(default)]
    rotate: Vec<Turn>,
}

pub const CYLINDER: Tool = Tool {
    name: "cylinder",
    description: "Lay a cylinder standing on XY about the origin, its axis along Z, in the active part; it joins the part's body.",
    schema: || schema(json!({ "radius": amount_schema("Radius, mm"), "height": amount_schema("Height along Z, mm") }), &["radius", "height"]),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: CylinderArgs = tool::args(arguments)?;
        let r = size(ctx, &a.radius, "radius")?;
        let h = size(ctx, &a.height, "height")?;
        let (radius, height) = (r.value, h.value);
        let laying = Laying { step: "cmd-cylinder", dims: vec![Dim { key: "r", read: r }, Dim { key: "h", read: h }], place: Placement { at: a.at, rotate: a.rotate } };
        lay(ctx, laying, |p| p.add_cylinder(radius, height))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SphereArgs {
    radius: Amount,
    at: Option<[f64; 3]>,
    #[serde(default)]
    rotate: Vec<Turn>,
}

pub const SPHERE: Tool = Tool {
    name: "sphere",
    description: "Lay a sphere about the origin in the active part; it joins the part's body.",
    schema: || schema(json!({ "radius": amount_schema("Radius, mm") }), &["radius"]),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SphereArgs = tool::args(arguments)?;
        let r = size(ctx, &a.radius, "radius")?;
        let radius = r.value;
        let laying = Laying { step: "cmd-sphere", dims: vec![Dim { key: "r", read: r }], place: Placement { at: a.at, rotate: a.rotate } };
        lay(ctx, laying, |p| p.add_sphere(radius))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConeArgs {
    bottom_radius: Amount,
    #[serde(default = "no_top")]
    top_radius: Amount,
    height: Amount,
    at: Option<[f64; 3]>,
    #[serde(default)]
    rotate: Vec<Turn>,
}

fn no_top() -> Amount {
    Amount::Number(0.0)
}

pub const CONE: Tool = Tool {
    name: "cone",
    description: "Lay a cone standing on XY about the origin, from bottom_radius at Z 0 to top_radius (0 by default: a point) at height; a top above 0 gives a frustum. It joins the part's body.",
    schema: || {
        schema(
            json!({ "bottom_radius": amount_schema("Radius at Z 0, mm"), "top_radius": amount_schema("Radius at the top, mm; 0 for a point"), "height": amount_schema("Height along Z, mm") }),
            &["bottom_radius", "height"],
        )
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ConeArgs = tool::args(arguments)?;
        let r1 = size(ctx, &a.bottom_radius, "bottom_radius")?;
        let r2 = size_or_none(ctx, &a.top_radius, "top_radius")?;
        let h = size(ctx, &a.height, "height")?;
        let (bottom, top, height) = (r1.value, r2.value, h.value);
        let laying = Laying { step: "cmd-cone", dims: vec![Dim { key: "r1", read: r1 }, Dim { key: "r2", read: r2 }, Dim { key: "h", read: h }], place: Placement { at: a.at, rotate: a.rotate } };
        lay(ctx, laying, |p| p.add_cone(bottom, top, height))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TorusArgs {
    ring_radius: Amount,
    tube_radius: Amount,
    at: Option<[f64; 3]>,
    #[serde(default)]
    rotate: Vec<Turn>,
}

pub const TORUS: Tool = Tool {
    name: "torus",
    description: "Lay a torus lying on XY about the origin: ring_radius from the axis to the middle of the tube, tube_radius of the tube, thinner than the ring. It joins the part's body.",
    schema: || {
        schema(
            json!({ "ring_radius": amount_schema("From the axis to the middle of the tube, mm"), "tube_radius": amount_schema("Radius of the tube, mm; less than ring_radius") }),
            &["ring_radius", "tube_radius"],
        )
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: TorusArgs = tool::args(arguments)?;
        let ring = size(ctx, &a.ring_radius, "ring_radius")?;
        let tube = size(ctx, &a.tube_radius, "tube_radius")?;
        // a tube as thick as its ring or thicker passes through itself: the kernel builds it and the body is not a
        // solid one can print, so it is refused here, as the field of the window refuses it
        if tube.value >= ring.value - 1e-3 {
            return Err(Refusal::new("tube-too-thick", &format!("tube_radius {} is not less than ring_radius {}: the torus would pass through itself.", tube.value, ring.value), Stage::Validate));
        }
        let (major, minor) = (ring.value, tube.value);
        let laying = Laying { step: "cmd-torus", dims: vec![Dim { key: "major", read: ring }, Dim { key: "minor", read: tube }], place: Placement { at: a.at, rotate: a.rotate } };
        lay(ctx, laying, |p| p.add_torus(major, minor))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrismArgs {
    radius: Amount,
    height: Amount,
    sides: u32,
    at: Option<[f64; 3]>,
    #[serde(default)]
    rotate: Vec<Turn>,
}

pub const PRISM: Tool = Tool {
    name: "prism",
    description: "Lay a regular prism standing on XY about the origin: sides (3 to 64) in a circle of radius, the first corner on +X, height along Z. It joins the part's body.",
    schema: || {
        schema(
            json!({ "radius": amount_schema("Radius of the circle through the corners, mm"), "height": amount_schema("Height along Z, mm"), "sides": { "type": "integer", "minimum": 3, "maximum": 64 } }),
            &["radius", "height", "sides"],
        )
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: PrismArgs = tool::args(arguments)?;
        if !(3..=64).contains(&a.sides) {
            return Err(Refusal::new("bad-size", &format!("sides is {}; from 3 to 64 are taken.", a.sides), Stage::Validate));
        }
        let r = size(ctx, &a.radius, "radius")?;
        let h = size(ctx, &a.height, "height")?;
        let (radius, sides, height) = (r.value, a.sides, h.value);
        let laying = Laying { step: "cmd-prism", dims: vec![Dim { key: "r", read: r }, Dim { key: "h", read: h }], place: Placement { at: a.at, rotate: a.rotate } };
        lay(ctx, laying, |p| p.add_prism(radius, sides, height))
    },
};
