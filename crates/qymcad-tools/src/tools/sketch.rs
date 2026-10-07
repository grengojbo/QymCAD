//! SKETCHES: a flat drawing on a plane, a datum or a face, made of lines, arcs and circles held by constraints and
//! dimensions - the profile every extrusion, revolution and cut is made of.
//!
//! A model draws by naming: an entity laid with `as: "r"` is reached afterwards as `r` and its points as `r.a`,
//! `r.c` or, for a rectangle, `r.bl` and `r.bottom`. Constraints and dimensions refer to those names, to the points
//! and entities by their keys from `sketch_info`, and to `origin`, `x_axis` and `y_axis`. The answer says how far the
//! drawing is from fully held (`dof`), which constraints fight, and which closed contours it makes - the contours an
//! extrusion takes.

use std::collections::BTreeMap;

use qymcad_core::feature::{BasePlane, Purpose, SketchPlane, Winding};
use qymcad_core::geom::Point2;
use qymcad_core::model::{Constraint, EntityKind, Id, PlaneDef, Project, WorkPlane};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::{self, Amount, BodyRef, QueryArg};
use crate::tool::{self, Ctx, Refusal, Stage, Tool};
use crate::tools::doc::{answer, outcome};

/// A base plane of the world.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum World {
    Xy,
    Xz,
    Yz,
}

impl World {
    fn base(self) -> BasePlane {
        match self {
            World::Xy => BasePlane::XY,
            World::Xz => BasePlane::XZ,
            World::Yz => BasePlane::YZ,
        }
    }
}

/// Where a sketch stands.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum PlaneArg {
    World(World),
    Datum { datum: Id },
    Face { body: BodyRef, face: QueryArg },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateArgs {
    plane: PlaneArg,
    offset: Option<Amount>,
}

/// The plane the sketch stands on, an offset datum laid first when `offset` asks for one; the datum's distance keeps
/// its expression.
fn lay_plane(p: &mut Project, plane: &SketchPlane, offset: Option<&args::Read>) -> SketchPlane {
    let Some(off) = offset else { return *plane };
    let def = match plane {
        SketchPlane::World(base) => PlaneDef::OffsetBase { base: *base, dist: off.value },
        SketchPlane::Datum(id) => PlaneDef::OffsetPlane { plane: *id, dist: off.value },
        SketchPlane::Face(body, key) => PlaneDef::OffsetFace { body: *body, face: *key, dist: off.value },
    };
    let datum = p.add_plane(WorkPlane { def, ..Default::default() });
    p.set_feat_dim(datum, "dist", off.expr.clone().unwrap_or_default());
    SketchPlane::Datum(datum)
}

pub const CREATE_SKETCH: Tool = Tool {
    name: "create_sketch",
    description: "Start a sketch in the active part (in an assembly a part is made for it): on a world plane \"xy\", \"xz\" or \"yz\", on a datum {\"datum\": key}, or on a flat face {\"body\": {...}, \"face\": query} that finds one face. offset (mm, or an expression) lays a datum that far along the normal first and sketches on it. Then sketch_add draws in it.",
    schema: || {
        json!({ "type": "object", "properties": {
            "plane": { "description": "\"xy\", \"xz\", \"yz\"; {\"datum\": key}; or {\"body\": {\"body\": key} or {\"part\": key}, \"face\": query}.", "oneOf": [
                { "type": "string", "enum": ["xy", "xz", "yz"] },
                { "type": "object", "properties": { "datum": { "type": "integer" } }, "required": ["datum"], "additionalProperties": false },
                { "type": "object", "properties": { "body": args::body_schema(), "face": args::query_schema() }, "required": ["body", "face"], "additionalProperties": false },
            ] },
            "offset": args::amount_schema("Distance of an offset datum along the normal, mm"),
        }, "required": ["plane"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: CreateArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let plane = match &a.plane {
            PlaneArg::World(w) => SketchPlane::World(w.base()),
            PlaneArg::Datum { datum } => {
                if !project.planes.iter().any(|pl| pl.id == *datum) {
                    return Err(Refusal::new("no-datum", &format!("There is no datum plane {datum}."), Stage::Validate).with_hint("Read get_document for the features of kind Plane."));
                }
                SketchPlane::Datum(*datum)
            }
            PlaneArg::Face { body, face } => {
                let body = body.resolve(project)?;
                SketchPlane::Face(body, args::face_key(project, body, face)?)
            }
        };
        let offset = a.offset.as_ref().map(|o| o.read(project, "offset")).transpose()?;
        let made = ctx
            .doc
            .edit("status-new-sketch", |p| {
                let _ = p.part_to_draw_in();
                // a face of another part is read across the parts only once it is named as an input, as the window
                // names it, or the rebuild keeps the parts apart and the sketch finds no face
                if let SketchPlane::Face(body, key) = &plane {
                    let consumer = p.active_ctx();
                    if p.body_owner(*body).is_some_and(|owner| owner != consumer) {
                        p.add_external_face_ref(consumer, *body, *key);
                    }
                }
                let stands = lay_plane(p, &plane, offset.as_ref());
                let number = p.sketches.len() + 1;
                let si = p.new_sketch(format!("name-sketch-n#{number}"));
                p.sketches[si].plane = stands;
                let sid = p.sketches[si].id;
                let _ = p.add_sketch_node(sid, format!("name-sketch-n#{number}"));
                p.regen_sketch(si);
                Ok(Made { sketch: sid, datum: match stands {
                    SketchPlane::Datum(id) if a.offset.is_some() => Some(id),
                    _ => None,
                } })
            })
            .map_err(tool::doc_refusal)?;
        let mut out = answer("sketch", json!(made.sketch));
        if let Some(d) = made.datum {
            out.insert("datum".into(), json!(d));
        }
        if let Some(f) = ctx.doc.project().sketch_index(made.sketch).and_then(|si| ctx.doc.project().sketch_frame(si)) {
            out.insert("frame".into(), json!({ "origin": f.origin, "x": f.x, "y": f.y }));
        }
        Ok(outcome(ctx, out))
    },
};

/// The sketch made and the datum laid for it, if one was.
struct Made {
    sketch: Id,
    datum: Option<Id>,
}

/// Whether an entity is part of the profile or a support only.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PurposeArg {
    #[default]
    Real,
    Construction,
}

impl PurposeArg {
    fn core(self) -> Purpose {
        match self {
            PurposeArg::Real => Purpose::Real,
            PurposeArg::Construction => Purpose::Construction,
        }
    }
}

/// The way an arc turns from its start to its end.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TurnArg {
    #[default]
    Ccw,
    Cw,
}

/// AN ENTITY TO DRAW, in sketch millimetres on the sketch's own axes.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Shape {
    Point {
        at: [f64; 2],
        #[serde(rename = "as")]
        name: Option<String>,
    },
    Line {
        from: [f64; 2],
        to: [f64; 2],
        #[serde(rename = "as")]
        name: Option<String>,
        #[serde(default)]
        purpose: PurposeArg,
    },
    Rect {
        from: [f64; 2],
        to: [f64; 2],
        #[serde(rename = "as")]
        name: Option<String>,
        #[serde(default)]
        purpose: PurposeArg,
    },
    Circle {
        centre: [f64; 2],
        radius: f64,
        #[serde(rename = "as")]
        name: Option<String>,
        #[serde(default)]
        purpose: PurposeArg,
    },
    Arc {
        centre: [f64; 2],
        from: [f64; 2],
        to: [f64; 2],
        #[serde(default)]
        turn: TurnArg,
        #[serde(rename = "as")]
        name: Option<String>,
        #[serde(default)]
        purpose: PurposeArg,
    },
    Polygon {
        centre: [f64; 2],
        corner: [f64; 2],
        sides: u32,
        #[serde(rename = "as")]
        name: Option<String>,
        #[serde(default)]
        purpose: PurposeArg,
    },
    Slot {
        from: [f64; 2],
        to: [f64; 2],
        radius: f64,
        #[serde(default)]
        purpose: PurposeArg,
    },
}

/// A point, a line or a circle, by a name of this call, a key, or `origin`, `x_axis`, `y_axis`.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum Name {
    Key(Id),
    Word(String),
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Name::Key(k) => write!(f, "{k}"),
            Name::Word(w) => write!(f, "\"{w}\""),
        }
    }
}

/// A CONSTRAINT: a relation between points, lines and circles, held by the solver.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Relation {
    Horizontal(Name),
    Vertical(Name),
    Fixed(Name),
    Coincident([Name; 2]),
    Parallel([Name; 2]),
    Perpendicular([Name; 2]),
    Collinear([Name; 2]),
    /// Two lines of one length, or two circles of one radius.
    Equal([Name; 2]),
    Concentric([Name; 2]),
    /// A line and a circle, or two circles touching from outside.
    Tangent([Name; 2]),
    Midpoint {
        point: Name,
        line: Name,
    },
    /// A point on a line (its infinite line) or on a circle.
    On {
        point: Name,
        of: Name,
    },
    Symmetric {
        points: [Name; 2],
        axis: Name,
    },
}

/// How a distance between two points is measured.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Along {
    #[default]
    Aligned,
    X,
    Y,
}

impl Along {
    fn axis(self) -> u8 {
        match self {
            Along::Aligned => 0,
            Along::X => 1,
            Along::Y => 2,
        }
    }
}

/// A DIMENSION: a size the solver holds, a number or an expression; `name` makes it a name formulas read.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Measure {
    Distance {
        between: [Name; 2],
        value: Amount,
        #[serde(default)]
        along: Along,
        name: Option<String>,
    },
    Length {
        line: Name,
        value: Amount,
        name: Option<String>,
    },
    Radius {
        circle: Name,
        value: Amount,
        name: Option<String>,
    },
    Diameter {
        circle: Name,
        value: Amount,
        name: Option<String>,
    },
    Angle {
        lines: [Name; 2],
        value: Amount,
        name: Option<String>,
    },
    /// From a point square to a line.
    Offset {
        point: Name,
        line: Name,
        value: Amount,
        name: Option<String>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddArgs {
    sketch: Id,
    #[serde(default)]
    entities: Vec<Shape>,
    #[serde(default)]
    constraints: Vec<Value>,
    #[serde(default)]
    dimensions: Vec<Value>,
}

/// AN ITEM AS IT WAS WRITTEN and as it reads: the answer names a constraint left out in the model's own words.
struct Written<T> {
    raw: Value,
    item: T,
}

/// Read every item of `list`, a refusal naming the first that does not read by its place.
fn read_each<T: serde::de::DeserializeOwned>(list: &[Value], what: &str) -> Result<Vec<Written<T>>, Refusal> {
    list.iter()
        .enumerate()
        .map(|(i, raw)| match serde_json::from_value::<T>(raw.clone()) {
            Ok(item) => Ok(Written { raw: raw.clone(), item }),
            Err(e) => Err(Refusal::new("arguments", &format!("{what}[{i}] {raw}: {e}"), Stage::Validate).with_hint("Compare it with the forms in the tool's description.")),
        })
        .collect()
}

/// A call read whole before the document is touched.
struct Plan {
    entities: Vec<Shape>,
    constraints: Vec<Written<Relation>>,
    dimensions: Vec<Written<Measure>>,
}

/// What a name stands for in the solver's terms.
#[derive(Clone, Copy, Debug)]
enum Thing {
    Point(Id),
    /// A line by its two end points, in order.
    Line {
        ends: [Id; 2],
    },
    /// A circle or an arc by its centre, which carries its radius.
    Circle {
        centre: Id,
    },
}

impl Thing {
    fn json(self) -> Value {
        match self {
            Thing::Point(p) => json!({ "point": p }),
            Thing::Line { ends } => json!({ "line": ends }),
            Thing::Circle { centre } => json!({ "circle_centre": centre }),
        }
    }
}

/// THE DRAWING OF ONE CALL: the sketch, and the names this call gave.
struct Pad<'a> {
    p: &'a mut Project,
    si: usize,
    names: BTreeMap<String, Thing>,
}

fn pt(v: [f64; 2]) -> Point2 {
    Point2::new(v[0], v[1])
}

fn refused(code: &str, message: String) -> Refusal {
    Refusal::new(code, &message, Stage::Validate)
}

impl Pad<'_> {
    fn name(&mut self, name: &Option<String>, thing: Thing) -> Result<(), Refusal> {
        if let Some(n) = name {
            if matches!(n.as_str(), "origin" | "x_axis" | "y_axis") || n.contains('.') {
                return Err(refused("unfit-name", format!("\"{n}\" cannot name an entity: it is reserved or holds a dot.")));
            }
            if self.names.insert(n.clone(), thing).is_some() {
                return Err(refused("taken-name", format!("\"{n}\" names two entities in this call.")));
            }
        }
        Ok(())
    }

    fn sub(&mut self, name: &Option<String>, part: &str, thing: Thing) {
        if let Some(n) = name {
            self.names.insert(format!("{n}.{part}"), thing);
        }
    }

    fn line_ends(&self, eid: Id) -> Option<[Id; 2]> {
        self.p.sketches[self.si].entities.iter().find(|e| e.id == eid).and_then(|e| match e.kind {
            EntityKind::Line { a, b } => Some([a, b]),
            _ => None,
        })
    }

    fn last_entity(&self) -> Option<Id> {
        self.p.sketches[self.si].entities.last().map(|e| e.id)
    }

    /// DRAW ONE ENTITY and name it and its points.
    fn draw(&mut self, shape: &Shape) -> Result<(), Refusal> {
        let si = self.si;
        match shape {
            Shape::Point { at, name } => {
                let id = self.p.sketch_point_at(si, at[0], at[1], 1e-9);
                self.name(name, Thing::Point(id))
            }
            Shape::Line { from, to, name, purpose } => {
                if from == to {
                    return Err(refused("bad-size", format!("A line from {from:?} to the same point has no length.")));
                }
                let eid = self.p.add_line_entity(si, from[0], from[1], to[0], to[1], purpose.core());
                let ends = self.line_ends(eid).ok_or_else(|| refused("kernel-panic", "The line was not laid.".into()))?;
                self.name(name, Thing::Line { ends })?;
                self.sub(name, "a", Thing::Point(ends[0]));
                self.sub(name, "b", Thing::Point(ends[1]));
                Ok(())
            }
            Shape::Rect { from, to, name, purpose } => {
                if from[0] == to[0] || from[1] == to[1] {
                    return Err(refused("bad-size", format!("A rectangle from {from:?} to {to:?} has no area.")));
                }
                // the sides come bottom, right, top, left from the lower left corner (`add_rect_entity`)
                let sides = self.p.add_rect_entity(si, from[0], from[1], to[0], to[1], purpose.core());
                let ends: Vec<[Id; 2]> = sides.iter().filter_map(|e| self.line_ends(*e)).collect();
                let [bottom, right, top, left] = ends.as_slice() else { return Err(refused("kernel-panic", "The rectangle was not laid.".into())) };
                let whole = Thing::Line { ends: *bottom };
                self.name(name, whole)?;
                self.sub(name, "bottom", Thing::Line { ends: *bottom });
                self.sub(name, "right", Thing::Line { ends: *right });
                self.sub(name, "top", Thing::Line { ends: *top });
                self.sub(name, "left", Thing::Line { ends: *left });
                self.sub(name, "bl", Thing::Point(bottom[0]));
                self.sub(name, "br", Thing::Point(bottom[1]));
                self.sub(name, "tr", Thing::Point(top[0]));
                self.sub(name, "tl", Thing::Point(top[1]));
                Ok(())
            }
            Shape::Circle { centre, radius, name, purpose } => {
                if radius.is_nan() || *radius <= 0.0 {
                    return Err(refused("bad-size", format!("A circle of radius {radius} is not drawn.")));
                }
                let eid = self.p.add_circle_entity(si, centre[0], centre[1], *radius, purpose.core());
                let c = self.circle_centre(eid)?;
                self.name(name, Thing::Circle { centre: c })?;
                self.sub(name, "c", Thing::Point(c));
                Ok(())
            }
            Shape::Arc { centre, from, to, turn, name, purpose } => {
                let winding = match turn {
                    TurnArg::Ccw => Winding::Ccw,
                    TurnArg::Cw => Winding::Cw,
                };
                self.p.add_arc_entity(si, pt(*centre), pt(*from), pt(*to), winding, purpose.core());
                let kind = self.last_entity().and_then(|e| self.p.sketches[si].entities.iter().find(|x| x.id == e)).map(|e| e.kind);
                let Some(EntityKind::Arc { center, a, b, .. }) = kind else { return Err(refused("bad-size", format!("An arc about {centre:?} from {from:?} to {to:?} was not drawn."))) };
                self.name(name, Thing::Circle { centre: center })?;
                self.sub(name, "c", Thing::Point(center));
                self.sub(name, "a", Thing::Point(a));
                self.sub(name, "b", Thing::Point(b));
                Ok(())
            }
            Shape::Polygon { centre, corner, sides, name, purpose } => {
                if !(3..=64).contains(sides) {
                    return Err(refused("bad-size", format!("sides is {sides}; from 3 to 64 are taken.")));
                }
                let (centre_id, side_ids) = self.p.add_polygon_param(si, pt(*centre), pt(*corner), *sides, purpose.core());
                self.name(name, Thing::Point(centre_id))?;
                self.sub(name, "c", Thing::Point(centre_id));
                for (i, e) in side_ids.iter().enumerate() {
                    if let Some(ends) = self.line_ends(*e) {
                        self.sub(name, &i.to_string(), Thing::Line { ends });
                    }
                }
                Ok(())
            }
            Shape::Slot { from, to, radius, purpose } => {
                if radius.is_nan() || *radius <= 0.0 || from == to {
                    return Err(refused("bad-size", format!("A slot from {from:?} to {to:?} of radius {radius} is not drawn.")));
                }
                self.p.add_slot_entity(si, pt(*from), pt(*to), *radius, purpose.core());
                Ok(())
            }
        }
    }

    fn circle_centre(&self, eid: Id) -> Result<Id, Refusal> {
        self.p.sketches[self.si]
            .entities
            .iter()
            .find(|e| e.id == eid)
            .and_then(|e| match e.kind {
                EntityKind::Circle { center, .. } | EntityKind::Arc { center, .. } => Some(center),
                _ => None,
            })
            .ok_or_else(|| refused("kernel-panic", "The circle was not laid.".into()))
    }

    /// WHAT A NAME STANDS FOR: a name of this call, a point or entity key of the sketch, or the frame.
    fn find(&mut self, n: &Name) -> Result<Thing, Refusal> {
        let si = self.si;
        match n {
            Name::Word(w) => match w.as_str() {
                "origin" => Ok(Thing::Point(self.p.ensure_origin(si))),
                "x_axis" => Ok(Thing::Line { ends: self.p.ensure_axis(si, 0).into() }),
                "y_axis" => Ok(Thing::Line { ends: self.p.ensure_axis(si, 1).into() }),
                _ => self.names.get(w).copied().ok_or_else(|| {
                    let known: Vec<&String> = self.names.keys().collect();
                    refused("unknown-name", format!("{n} names nothing in this call; the names are {known:?}, origin, x_axis, y_axis, or a key from sketch_info."))
                }),
            },
            Name::Key(k) => {
                let s = &self.p.sketches[si];
                if s.points.iter().any(|p| p.id == *k) {
                    return Ok(Thing::Point(*k));
                }
                match s.entities.iter().find(|e| e.id == *k).map(|e| e.kind) {
                    Some(EntityKind::Line { a, b }) => Ok(Thing::Line { ends: [a, b] }),
                    Some(EntityKind::Circle { center, .. } | EntityKind::Arc { center, .. }) => Ok(Thing::Circle { centre: center }),
                    _ => Err(refused("unknown-name", format!("{k} is no point, line or circle of sketch {}.", s.id))),
                }
            }
        }
    }

    fn point(&mut self, n: &Name) -> Result<Id, Refusal> {
        match self.find(n)? {
            Thing::Point(p) => Ok(p),
            other => Err(refused("wrong-kind", format!("{n} is {}, where a point is wanted.", other.json()))),
        }
    }

    fn line(&mut self, n: &Name) -> Result<[Id; 2], Refusal> {
        match self.find(n)? {
            Thing::Line { ends } => Ok(ends),
            other => Err(refused("wrong-kind", format!("{n} is {}, where a line is wanted.", other.json()))),
        }
    }

    fn circle(&mut self, n: &Name) -> Result<Id, Refusal> {
        match self.find(n)? {
            Thing::Circle { centre } => Ok(centre),
            other => Err(refused("wrong-kind", format!("{n} is {}, where a circle or an arc is wanted.", other.json()))),
        }
    }

    /// The radius a circle or an arc has now.
    fn radius_now(&self, centre: Id) -> f64 {
        let s = &self.p.sketches[self.si];
        let at = |id: Id| s.points.iter().find(|p| p.id == id).map(|p| (p.x, p.y));
        s.entities
            .iter()
            .find_map(|e| match e.kind {
                EntityKind::Circle { center, r } if center == centre => Some(r),
                EntityKind::Arc { center, a, .. } if center == centre => at(center).zip(at(a)).map(|(c, p)| (p.0 - c.0).hypot(p.1 - c.1)),
                _ => None,
            })
            .unwrap_or(0.0)
    }

    /// The constraint of a relation.
    fn relation(&mut self, r: &Relation) -> Result<Constraint, Refusal> {
        Ok(match r {
            Relation::Horizontal(n) => {
                let [a, b] = self.line(n)?;
                Constraint::Horizontal { a, b }
            }
            Relation::Vertical(n) => {
                let [a, b] = self.line(n)?;
                Constraint::Vertical { a, b }
            }
            Relation::Fixed(n) => Constraint::Fixed { p: self.point(n)? },
            Relation::Coincident([m, n]) => Constraint::Coincident { a: self.point(m)?, b: self.point(n)? },
            Relation::Parallel([m, n]) => {
                let ([a, b], [c, d]) = (self.line(m)?, self.line(n)?);
                Constraint::Parallel { a, b, c, d }
            }
            Relation::Perpendicular([m, n]) => {
                let ([a, b], [c, d]) = (self.line(m)?, self.line(n)?);
                Constraint::Perpendicular { a, b, c, d }
            }
            Relation::Collinear([m, n]) => {
                let ([a, b], [c, d]) = (self.line(m)?, self.line(n)?);
                Constraint::Collinear { a, b, c, d }
            }
            Relation::Equal([m, n]) => match (self.find(m)?, self.find(n)?) {
                (Thing::Line { ends: [a, b] }, Thing::Line { ends: [c, d] }) => Constraint::Equal { a, b, c, d },
                (Thing::Circle { centre: c1 }, Thing::Circle { centre: c2 }) => Constraint::EqualRadius { c1, c2 },
                _ => return Err(refused("wrong-kind", format!("equal takes two lines or two circles: {m} and {n}."))),
            },
            Relation::Concentric([m, n]) => Constraint::Concentric { c1: self.circle(m)?, c2: self.circle(n)? },
            Relation::Tangent([m, n]) => match (self.find(m)?, self.find(n)?) {
                (Thing::Line { ends: [a, b] }, Thing::Circle { centre }) | (Thing::Circle { centre }, Thing::Line { ends: [a, b] }) => {
                    Constraint::Tangent { a, b, c: centre, r: self.radius_now(centre) }
                }
                (Thing::Circle { centre: c1 }, Thing::Circle { centre: c2 }) => Constraint::CircleTangent { c1, c2, external: true },
                _ => return Err(refused("wrong-kind", format!("tangent takes a line and a circle, or two circles: {m} and {n}."))),
            },
            Relation::Midpoint { point, line } => {
                let p = self.point(point)?;
                let [a, b] = self.line(line)?;
                Constraint::Midpoint { p, a, b }
            }
            Relation::On { point, of } => {
                let p = self.point(point)?;
                match self.find(of)? {
                    Thing::Line { ends: [a, b] } => Constraint::PointOnLine { p, a, b },
                    Thing::Circle { centre } => Constraint::PointOnCircle { p, c: centre },
                    Thing::Point(_) => return Err(refused("wrong-kind", format!("{of} is a point; a point lies on a line or a circle."))),
                }
            }
            Relation::Symmetric { points: [m, n], axis } => {
                let (a, b) = (self.point(m)?, self.point(n)?);
                let [la, lb] = self.line(axis)?;
                Constraint::Symmetric { a, b, la, lb }
            }
        })
    }

    /// The constraint of a dimension, and the name it is to carry.
    fn measure(&mut self, m: &Measure) -> Result<Sized, Refusal> {
        let read = |p: &Project, v: &Amount, field: &str| -> Result<args::Read, Refusal> {
            let r = v.read(p, field)?;
            if r.value > 0.0 {
                Ok(r)
            } else {
                Err(refused("bad-size", format!("{field} is {}; it must be more than 0.", r.value)))
            }
        };
        let (c, name) = match m {
            Measure::Distance { between: [m, n], value, along, name } => {
                let (a, b) = (self.point(m)?, self.point(n)?);
                let r = read(self.p, value, "distance")?;
                (Constraint::Distance { a, b, d: r.value, off: 0.0, expr: r.expr.unwrap_or_default(), driven: false, axis: along.axis(), at: None }, name)
            }
            Measure::Length { line, value, name } => {
                let [a, b] = self.line(line)?;
                let r = read(self.p, value, "length")?;
                (Constraint::Distance { a, b, d: r.value, off: 0.0, expr: r.expr.unwrap_or_default(), driven: false, axis: 0, at: None }, name)
            }
            Measure::Radius { circle, value, name } | Measure::Diameter { circle, value, name } => {
                let c = self.circle(circle)?;
                let diam = matches!(m, Measure::Diameter { .. });
                let r = read(self.p, value, if diam { "diameter" } else { "radius" })?;
                (Constraint::Diameter { c, d: r.value, off: 0.0, expr: r.expr.unwrap_or_default(), driven: false, diam, at: None }, name)
            }
            Measure::Angle { lines: [m, n], value, name } => {
                let ([a, b], [c, d]) = (self.line(m)?, self.line(n)?);
                let r = read(self.p, value, "angle")?;
                (Constraint::AngleLines { a, b, c, d, deg: r.value, expr: r.expr.unwrap_or_default(), driven: false, off: 0.0, at: None }, name)
            }
            Measure::Offset { point, line, value, name } => {
                let p = self.point(point)?;
                let [a, b] = self.line(line)?;
                let r = read(self.p, value, "offset")?;
                (Constraint::DistancePL { p, a, b, d: r.value, off: 0.0, expr: r.expr.unwrap_or_default(), driven: false, at: None }, name)
            }
        };
        Ok(Sized { constraint: c, name: name.clone() })
    }
}

/// A dimension and the name it carries.
struct Sized {
    constraint: Constraint,
    name: Option<String>,
}

/// WHAT A SKETCH HOLDS NOW, as the model reads it.
fn account(p: &Project, si: usize) -> Value {
    let s = &p.sketches[si];
    let dof = p.sketch_dof(si);
    let conflicts: Vec<usize> = p.sketch_conflicts(si);
    let contours: Vec<Value> = s
        .contour_ids
        .iter()
        .filter_map(|cid| {
            let c = p.contours.get(p.contour_index(*cid)?)?;
            let pts = &c.points;
            let area = if c.closed && pts.len() > 2 {
                0.5 * (0..pts.len()).map(|i| pts[i].x * pts[(i + 1) % pts.len()].y - pts[(i + 1) % pts.len()].x * pts[i].y).sum::<f64>().abs()
            } else {
                0.0
            };
            Some(json!({ "id": cid, "closed": c.closed, "area": (area * 1e6).round() / 1e6, "profile": p.contour_profile_xy(*cid).is_some() }))
        })
        .collect();
    json!({ "sketch": s.id, "dof": dof.0, "redundant": dof.1, "conflicts": conflicts, "contours": contours })
}

fn sketch_at(ctx: &Ctx, sketch: Id) -> Result<usize, Refusal> {
    ctx.doc.project().sketch_index(sketch).ok_or_else(|| refused("no-sketch", format!("There is no sketch {sketch}.")).with_hint("create_sketch gives the key of a new one."))
}

pub const SKETCH_ADD: Tool = Tool {
    name: "sketch_add",
    description: "Draw in a sketch, as one undo step: entities first, then constraints, then dimensions, then the sketch is solved. Entities (sketch mm on the sketch axes), each an object of one key: {\"line\": {\"from\": [x,y], \"to\": [x,y], \"as\": \"l\"}} (points l.a, l.b), {\"rect\": {\"from\", \"to\", \"as\": \"r\"}} (sides r.bottom r.right r.top r.left, corners r.bl r.br r.tr r.tl), {\"circle\": {\"centre\", \"radius\", \"as\": \"c\"}} (centre c.c), {\"arc\": {\"centre\", \"from\", \"to\", \"turn\": \"ccw\"|\"cw\", \"as\"}} (c, a, b), {\"polygon\": {\"centre\", \"corner\", \"sides\", \"as\"}} (sides p.0, p.1, ...), {\"slot\": {\"from\", \"to\", \"radius\"}}, {\"point\": {\"at\", \"as\"}}; \"purpose\": \"construction\" keeps one out of the profile. Constraints: {\"horizontal\": line}, {\"vertical\": line}, {\"fixed\": point}, {\"coincident\": [p, q]}, {\"parallel\"|\"perpendicular\"|\"collinear\": [l, m]}, {\"equal\": [l, m] or [c, d]}, {\"concentric\": [c, d]}, {\"tangent\": [line, circle] or [c, d]}, {\"midpoint\": {\"point\", \"line\"}}, {\"on\": {\"point\", \"of\"}}, {\"symmetric\": {\"points\": [p, q], \"axis\": line}}. Dimensions, a value a number or an expression, \"name\" making it a name formulas read: {\"distance\": {\"between\": [p, q], \"value\", \"along\": \"aligned\"|\"x\"|\"y\"}}, {\"length\": {\"line\", \"value\"}}, {\"radius\"|\"diameter\": {\"circle\", \"value\"}}, {\"angle\": {\"lines\": [l, m], \"value\"}} (degrees), {\"offset\": {\"point\", \"line\", \"value\"}}. A name is one given in this call, a key from sketch_info, origin, x_axis or y_axis. The answer gives the keys of the names, dof (0: fully held), the constraints left out as already implied, the dimensions made reference ones, conflicts, and the contours.",
    schema: || {
        json!({ "type": "object", "properties": {
            "sketch": { "type": "integer" },
            "entities": { "type": "array", "items": { "type": "object", "minProperties": 1, "maxProperties": 1 } },
            "constraints": { "type": "array", "items": { "type": "object", "minProperties": 1, "maxProperties": 1 } },
            "dimensions": { "type": "array", "items": { "type": "object", "minProperties": 1, "maxProperties": 1 } },
        }, "required": ["sketch"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: AddArgs = tool::args(arguments)?;
        let plan = Plan { entities: a.entities, constraints: read_each(&a.constraints, "constraints")?, dimensions: read_each(&a.dimensions, "dimensions")? };
        let si = sketch_at(ctx, a.sketch)?;
        let sid = a.sketch;
        let mut refusal = None;
        let drawn = ctx.doc.edit("sk-drawing", |p| {
            let mut pad = Pad { p, si, names: BTreeMap::new() };
            match draw_all(&mut pad, &plan) {
                Ok(d) => {
                    pad.p.solve_sketch(si);
                    pad.p.regen_sketch(si);
                    let names: serde_json::Map<String, Value> = pad.names.iter().map(|(k, t)| (k.clone(), t.json())).collect();
                    Ok(Drawn { names: Value::Object(names), left_out: d.left_out, reference: d.reference })
                }
                Err(r) => {
                    refusal = Some(r);
                    Err("refused".into())
                }
            }
        });
        let drawn = match (drawn, refusal) {
            (Ok(d), _) => d,
            (Err(_), Some(r)) => return Err(r),
            (Err(e), None) => return Err(tool::doc_refusal(e)),
        };
        let p = ctx.doc.project();
        let si = p.sketch_index(sid).ok_or_else(|| refused("no-sketch", format!("Sketch {sid} is gone.")))?;
        let mut out = answer("names", drawn.names);
        out.insert("left_out".into(), json!(drawn.left_out));
        out.insert("reference".into(), json!(drawn.reference));
        if let Value::Object(acc) = account(p, si) {
            out.extend(acc);
        }
        Ok(outcome(ctx, out))
    },
};

/// What the drawing left behind beside the names.
struct Drawn {
    names: Value,
    left_out: Vec<Value>,
    reference: Vec<Value>,
}

/// The constraints a call left out as implied, and the dimensions it made reference ones.
struct Trace {
    left_out: Vec<Value>,
    reference: Vec<Value>,
}

/// EVERYTHING OF ONE CALL, in order: entities, constraints, dimensions.
fn draw_all(pad: &mut Pad, a: &Plan) -> Result<Trace, Refusal> {
    for e in &a.entities {
        pad.draw(e)?;
    }
    let mut trace = Trace { left_out: Vec::new(), reference: Vec::new() };
    for w in &a.constraints {
        let c = pad.relation(&w.item)?;
        // a relation the drawing already holds is left out, not added twice: the solver would count it as a fight
        if !pad.p.add_constraint_if_independent(pad.si, c) {
            trace.left_out.push(w.raw.clone());
        }
    }
    for w in &a.dimensions {
        let sized = pad.measure(&w.item)?;
        let refs = Project::dim_refs(&sized.constraint).unwrap_or_default();
        pad.p.sketches[pad.si].constraints.push(sized.constraint);
        let ci = pad.p.sketches[pad.si].constraints.len() - 1;
        // a size the drawing already holds becomes a reference dimension, which shows the value and holds nothing, as
        // the window makes it
        if pad.p.auto_driven(pad.si, ci) {
            trace.reference.push(w.raw.clone());
        }
        if let Some(n) = &sized.name {
            let sid = pad.p.sketches[pad.si].id;
            if qymcad_core::drivers::check_ident(n).is_err() {
                return Err(refused("unfit-name", format!("The name \"{n}\" cannot be used in a formula: letters, digits and _ only, not starting with a digit.")));
            }
            if !pad.p.add_named_dim(n.clone(), sid, refs) {
                return Err(refused("taken-name", format!("The name \"{n}\" is taken by a parameter or another dimension.")).with_hint("Choose another name."));
            }
        }
    }
    Ok(trace)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InfoArgs {
    sketch: Id,
}

pub const SKETCH_INFO: Tool = Tool {
    name: "sketch_info",
    description: "Read a sketch: its plane, every point (key, x, y), every entity (key, kind, its point keys, construction), every constraint and dimension with its index, the named dimensions, dof, conflicts, and the contours (closed, area, whether an extrusion can take it).",
    schema: || json!({ "type": "object", "properties": { "sketch": { "type": "integer" } }, "required": ["sketch"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: InfoArgs = tool::args(arguments)?;
        let si = sketch_at(ctx, a.sketch)?;
        let p = ctx.doc.project();
        let s = &p.sketches[si];
        let points: Vec<Value> = s.points.iter().map(|q| json!({ "key": q.id, "x": q.x, "y": q.y })).collect();
        let entities: Vec<Value> = s
            .entities
            .iter()
            .map(|e| {
                let mut shown = match e.kind {
                    EntityKind::Line { a, b } => json!({ "kind": "line", "points": [a, b] }),
                    EntityKind::Arc { center, a, b, .. } => json!({ "kind": "arc", "points": [center, a, b] }),
                    EntityKind::Circle { center, r } => json!({ "kind": "circle", "points": [center], "radius": r }),
                    EntityKind::Ellipse { c, ma, mi } => json!({ "kind": "ellipse", "points": [c, ma, mi] }),
                };
                shown["key"] = json!(e.id);
                shown["construction"] = json!(e.construction);
                shown
            })
            .collect();
        let constraints: Vec<Value> = s.constraints.iter().enumerate().map(|(i, c)| json!({ "index": i, "is": format!("{c:?}") })).collect();
        let named: Vec<Value> = p
            .named_dims
            .iter()
            .filter(|n| matches!(&n.target, qymcad_core::model::DimTarget::Sketch { sketch, .. } if *sketch == s.id))
            .map(|n| json!({ "name": n.name, "value": p.named_dim_value(n) }))
            .collect();
        let mut out = answer("plane", json!(format!("{:?}", s.plane)));
        out.insert("points".into(), json!(points));
        out.insert("entities".into(), json!(entities));
        out.insert("constraints".into(), json!(constraints));
        out.insert("named".into(), json!(named));
        if let Value::Object(acc) = account(p, si) {
            out.extend(acc);
        }
        Ok(out)
    },
};
