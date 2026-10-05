//! THE WORDS A TOOL IS GIVEN its values and its geometry in: a size as a number or an expression, a body by its key or
//! by its part, and faces or edges as a query - "the top face", "every edge of the top face", "the faces looking along
//! +X" - rather than as numbers that stop meaning anything after the next rebuild.
//!
//! A query is the core's own (`refs::Query`) under names a model reads at once: the core's form carries Rust's
//! casing and nests a pair as a tuple, which a schema cannot describe and a reader has to count. The answer of a
//! resolution is the set the feature will act on, so a model sees what it is about to change before it changes it.

use qymcad_core::model::{Id, Project};
use qymcad_core::names::Role;
use qymcad_core::refs::{Axis, Cardinality, Fingerprint, Query, Ref};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::tool::{Refusal, Stage};

/// A SIZE: millimetres for a length, degrees for an angle - the field says which. A number is taken as it is; a
/// string is an expression over the parameters (`"w/2 + 1"`), and the feature keeps it, so a changed parameter moves
/// the feature with it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Amount {
    Number(f64),
    Expr(String),
}

/// A size read against the document: its value now, and the expression the feature keeps, if it was given one.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub value: f64,
    pub expr: Option<String>,
}

impl Amount {
    /// The value of the size in `project`; `field` names it in a refusal. A string that is a plain number is a number:
    /// a feature keeping "40" as a formula would show a formula where there is none.
    pub fn read(&self, project: &Project, field: &str) -> Result<Read, Refusal> {
        let text = match self {
            Amount::Number(v) => return finite(*v, field).map(|value| Read { value, expr: None }),
            Amount::Expr(s) => s.trim(),
        };
        if let Ok(v) = text.parse::<f64>() {
            return finite(v, field).map(|value| Read { value, expr: None });
        }
        match project.eval_expr(text) {
            Ok(v) => finite(v, field).map(|value| Read { value, expr: Some(text.to_string()) }),
            Err(e) => {
                let words = qymcad_i18n::error_words::error_text(&qymcad_core::errors::CoreError::Expr(e.clone()));
                Err(Refusal::new(e.key(), &format!("{field}: {words}"), Stage::Validate).with_hint("Read list_parameters for the names there are."))
            }
        }
    }
}

fn finite(v: f64, field: &str) -> Result<f64, Refusal> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(Refusal::new("not-a-number", &format!("{field} has no finite value."), Stage::Validate))
    }
}

/// A BODY: by its own key, or as the body of a part - the one body the part stands as.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BodyRef {
    Body(Id),
    Part(Id),
}

impl BodyRef {
    /// The body's key. A body taken into another (by a boolean, a cut, a hole) is refused: a feature on it would act on
    /// what the document no longer shows, and the answer names the body that took it.
    pub fn resolve(self, project: &Project) -> Result<Id, Refusal> {
        let consumed = project.consumed_bodies();
        match self {
            BodyRef::Body(id) => {
                if !project.bodies.iter().any(|b| b.id == id) {
                    return Err(Refusal::new("no-body", &format!("There is no body {id}."), Stage::Validate).with_hint("Read get_document for the bodies there are."));
                }
                if consumed.contains(&id) {
                    let mut stands = standing(project, project.body_owner(id));
                    let hint = match stands.pop() {
                        Some(last) if stands.is_empty() => format!("Body {last} stands in its place."),
                        _ => "Read get_document for the bodies that stand.".to_string(),
                    };
                    return Err(Refusal::new("consumed-body", &format!("Body {id} was taken into another body."), Stage::Validate).with_hint(&hint));
                }
                Ok(id)
            }
            BodyRef::Part(part) => {
                if !project.components.iter().any(|c| c.id == part && c.id != project.root) {
                    return Err(Refusal::new("no-part", &format!("There is no part {part}."), Stage::Validate).with_hint("Read get_document for the parts there are."));
                }
                match standing(project, Some(part)).as_slice() {
                    [one] => Ok(*one),
                    [] => Err(Refusal::new("no-body", &format!("Part {part} has no body yet."), Stage::Validate)),
                    many => Err(Refusal::new("several-bodies", &format!("Part {part} stands as {} bodies: {many:?}.", many.len()), Stage::Validate).with_hint("Name one as {\"body\": key}.")),
                }
            }
        }
    }
}

/// The bodies of `part` that stand on their own, in the order of the document.
fn standing(project: &Project, part: Option<Id>) -> Vec<Id> {
    let consumed = project.consumed_bodies();
    project.bodies.iter().filter(|b| !consumed.contains(&b.id) && part.is_some() && project.body_owner(b.id) == part).map(|b| b.id).collect()
}

/// The axis of `extreme`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisArg {
    X,
    Y,
    Z,
}

/// THE SHORT NAMES of the queries asked most. The sides are those of the world with Z up and the front looking
/// along -Y, as the views of the window name them.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Named {
    Top,
    Bottom,
    Front,
    Back,
    Left,
    Right,
    Largest,
    All,
}

impl Named {
    pub const ALL: [Named; 8] = [Named::Top, Named::Bottom, Named::Front, Named::Back, Named::Left, Named::Right, Named::Largest, Named::All];

    pub fn word(self) -> &'static str {
        match self {
            Named::Top => "top",
            Named::Bottom => "bottom",
            Named::Front => "front",
            Named::Back => "back",
            Named::Left => "left",
            Named::Right => "right",
            Named::Largest => "largest",
            Named::All => "all",
        }
    }

    fn query(self) -> Query {
        let extreme = |axis, max| Query::Extreme { axis, max };
        match self {
            Named::Top => extreme(Axis::Z, true),
            Named::Bottom => extreme(Axis::Z, false),
            Named::Front => extreme(Axis::Y, false),
            Named::Back => extreme(Axis::Y, true),
            Named::Left => extreme(Axis::X, false),
            Named::Right => extreme(Axis::X, true),
            Named::Largest => Query::Largest,
            // every element looks up or down or sideways: the two half-spaces of Z, each widened by half a degree,
            // take a side face (Z of its normal 0) in both and lose none to rounding - one cone of 180 deg would
            // test `dot >= -1` and drop a face whose normal comes out at -1 - 1e-16
            Named::All => {
                let half = |dir| Box::new(Query::Oriented { dir, tol_deg: 90.5 });
                Query::Union(half([0.0, 0.0, 1.0]), half([0.0, 0.0, -1.0]))
            }
        }
    }
}

/// A QUERY as the model writes it: a short name, or an object of one key.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum QueryArg {
    Named(Named),
    Built(Box<Built>),
}

/// The queries written out.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Built {
    /// Elements by the keys `list_faces` or `list_edges` gave.
    Ids(Vec<u32>),
    /// What a feature made, narrowed by role (`CapEnd` - the end cap of an extrusion, `Wall`, `Hole`, `Blend`, ...).
    OfFeature {
        feature: Id,
        #[serde(default)]
        role: Option<Role>,
    },
    /// Elements whose normal (for an edge: its direction) lies within `tol_deg` of `dir`.
    Facing {
        dir: [f64; 3],
        #[serde(default = "five_degrees")]
        tol_deg: f64,
    },
    /// The elements standing furthest along an axis.
    Extreme { axis: AxisArg, max: bool },
    /// The edges touching the faces of the query.
    Adjacent(QueryArg),
    /// The edges running on smoothly from the seed.
    TangentChain {
        seed: QueryArg,
        #[serde(default = "five_degrees")]
        tol_deg: f64,
    },
    /// The edges where a face of `one` meets a face of `other`.
    Between { one: QueryArg, other: QueryArg },
    /// Every element of any of the queries.
    Union(Vec<QueryArg>),
    /// The elements of `from` that `take` does not hold.
    Minus { from: QueryArg, take: QueryArg },
    /// The elements of `from` that `keep` holds as well.
    Filter { from: QueryArg, keep: QueryArg },
}

fn five_degrees() -> f64 {
    5.0
}

impl QueryArg {
    /// The core's query.
    pub fn query(&self) -> Query {
        let b = |q: &QueryArg| Box::new(q.query());
        let built = match self {
            QueryArg::Named(n) => return n.query(),
            QueryArg::Built(built) => built.as_ref(),
        };
        match built {
            Built::Ids(ids) => match ids.as_slice() {
                [one] => Query::Id(*one),
                many => Query::Ids(many.to_vec()),
            },
            Built::OfFeature { feature, role } => Query::OfFeature { feature: *feature, role: *role },
            Built::Facing { dir, tol_deg } => Query::Oriented { dir: *dir, tol_deg: *tol_deg },
            Built::Extreme { axis, max } => {
                let axis = match axis {
                    AxisArg::X => Axis::X,
                    AxisArg::Y => Axis::Y,
                    AxisArg::Z => Axis::Z,
                };
                Query::Extreme { axis, max: *max }
            }
            Built::Adjacent(faces) => Query::Adjacent(b(faces)),
            Built::TangentChain { seed, tol_deg } => Query::TangentChain { seed: b(seed), tol_deg: *tol_deg },
            Built::Between { one, other } => Query::Between(b(one), b(other)),
            Built::Union(all) => all.iter().map(QueryArg::query).reduce(|a, q| Query::Union(Box::new(a), Box::new(q))).unwrap_or(Query::Ids(Vec::new())),
            Built::Minus { from, take } => Query::Minus(b(from), b(take)),
            Built::Filter { from, keep } => Query::Filter(b(from), b(keep)),
        }
    }

    /// Whether the query asks for edges, whatever it names on the way: `adjacent`, `between` and `tangent_chain` do.
    pub fn yields_edges(&self) -> bool {
        self.query().yields_edges()
    }
}

/// HOW MANY the query must find. `one` refuses two - the model is told the count instead of the first being taken;
/// `some` refuses none; `any` takes whatever is there.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    One,
    Some,
    Any,
}

impl Expect {
    pub fn cardinality(self) -> Cardinality {
        match self {
            Expect::One => Cardinality::One,
            Expect::Some => Cardinality::Some,
            Expect::Any => Cardinality::Any,
        }
    }
}

/// What a query is asked of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Element {
    Faces,
    Edges,
}

impl Element {
    pub fn word(self) -> &'static str {
        match self {
            Element::Faces => "faces",
            Element::Edges => "edges",
        }
    }
}

/// The reference a feature keeps: the query and the count it expects, evaluated again on every rebuild.
pub fn reference(query: &QueryArg, expect: Expect) -> Ref {
    Ref { query: query.query(), expect: expect.cardinality(), hint: Fingerprint::default() }
}

/// RESOLVE a reference on `body` as it stands now: the keys of the faces or edges found, or a refusal with the count -
/// before any feature is laid, since a feature laid on a reference that finds nothing goes red with a code the model
/// cannot trace back to its query.
pub fn resolve(project: &Project, body: Id, r: &Ref, of: Element) -> Result<Vec<u32>, Refusal> {
    // the other way round is a question, not a slip: `top` asked of edges finds the topmost edges
    if r.query.yields_edges() && of == Element::Faces {
        return Err(
            Refusal::new("wrong-element", "The query asks for edges; faces are wanted here.", Stage::Validate).with_hint("adjacent, between and tangent_chain name edges; give the faces themselves.")
        );
    }
    let found = match of {
        Element::Faces => project.resolve_face_refs(body, r, of.word()),
        Element::Edges => project.resolve_edge_refs(body, r, of.word()),
    };
    found.map_err(|e| {
        let message = match &e {
            qymcad_core::refs::RefError::Lost { what, .. } => format!("The query found no {what} on body {body}."),
            qymcad_core::refs::RefError::Ambiguous { what, found } => format!("The query found {found} {what} on body {body}, where one was expected."),
        };
        Refusal::new(e.key(), &message, Stage::Resolve).with_hint("Read list_faces or list_edges, narrow the query, or set expect to some.")
    })
}

/// THE SCHEMA of a query, for the tools that take one: the short names, or an object of one key.
pub fn query_schema() -> Value {
    let names: Vec<&str> = Named::ALL.iter().map(|n| n.word()).collect();
    json!({
        "description": "Faces or edges by what they are, evaluated on the body as it stands. A short name, or an object of one key: {\"ids\": [k]}, {\"of_feature\": {\"feature\": key, \"role\": \"CapEnd\"}}, {\"facing\": {\"dir\": [0,0,1], \"tol_deg\": 5}}, {\"extreme\": {\"axis\": \"z\", \"max\": true}}, {\"adjacent\": q} (edges of faces), {\"tangent_chain\": {\"seed\": q}}, {\"between\": {\"one\": q, \"other\": q}}, {\"union\": [q, ...]}, {\"minus\": {\"from\": q, \"take\": q}}, {\"filter\": {\"from\": q, \"keep\": q}}. The sides: Z up, front looking along -Y.",
        "oneOf": [ { "type": "string", "enum": names }, { "type": "object", "minProperties": 1, "maxProperties": 1 } ],
    })
}

/// The schema of `expect`.
pub fn expect_schema() -> Value {
    json!({ "type": "string", "enum": ["one", "some", "any"], "description": "one: exactly one, two is refused; some: at least one; any: whatever there is." })
}

/// The schema of a size: a number, or an expression over the parameters.
pub fn amount_schema(what: &str) -> Value {
    json!({ "description": format!("{what}: a number, or an expression over the parameters (\"w/2\")."), "oneOf": [ { "type": "number" }, { "type": "string" } ] })
}

/// The schema of a body.
pub fn body_schema() -> Value {
    json!({ "description": "{\"body\": key}, or {\"part\": key} for the one body the part stands as.", "type": "object", "minProperties": 1, "maxProperties": 1 })
}

/// A FACE BY A QUERY THAT FINDS ONE: the key a feature keeps of it - its place among the body's faces, its centre and
/// normal now, and its name, by which the feature finds it again.
pub fn face_key(project: &Project, body: Id, face: &QueryArg) -> Result<qymcad_core::feature::FaceKey, Refusal> {
    let found = resolve(project, body, &reference(face, Expect::One), Element::Faces)?;
    let desc = found[0];
    let faces = project.bodies.iter().find(|b| b.id == body).map(|b| b.faces.as_slice()).unwrap_or_default();
    let (index, f) = faces.iter().enumerate().find(|(_, f)| f.id == desc).ok_or_else(|| Refusal::new("ref-lost", &format!("Face {desc} is not on body {body}."), Stage::Resolve))?;
    Ok(qymcad_core::feature::FaceKey { index: index as u32, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id })
}
