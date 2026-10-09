//! WHAT THE PERSON SELECTED IN THE OPEN WINDOW, so the model knows what "this" means: "make this rounding 0.2 mm
//! smaller", "a sketch on this face", "a hole in this plane". Each thing selected is told by its kind, where it is in
//! the world, and the key the other tools take for it; a face and an edge also by the feature that made them.
//!
//! Only the window knows what is selected; the program on a document of its own answers that there is no window. Names
//! are told as a person reads them ("Part 1"), not as the document stores them (a catalogue key).

use serde_json::{json, Value};

use qymcad_core::model::{Id, Project};

use crate::tool::{self, Answer, Ctx, Refusal, Seen, Stage, Tool};
use crate::tools::query::{edge_entry, face_entry};

/// Which end of an edge a selected corner is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum End {
    A,
    B,
}

/// ONE THING SELECTED IN THE WINDOW, by the keys the document keeps it under.
#[derive(Clone, Debug, PartialEq)]
pub enum Picked {
    Face { body: Id, face: u32 },
    Edge { body: Id, edge: u32 },
    Corner { body: Id, edge: u32, end: End },
    Plane(Id),
    Axis(Id),
    Point(Id),
    Sketch(Id),
    Contour(Id),
    Feature(Id),
    Part(Id),
    Body(Id),
    Joint(Id),
}

fn round(v: f64) -> f64 {
    let r = (v * 1e6).round() / 1e6;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

fn point(p: [f64; 3]) -> [f64; 3] {
    p.map(round)
}

/// THE ACCOUNT OF ONE THING SELECTED, with `use`: how the other tools are handed it.
fn describe(ctx: &Ctx, picked: &Picked) -> Value {
    let project = ctx.doc.project();
    match *picked {
        Picked::Face { body, face } => {
            let found = project.regen_faces.get(&body).and_then(|fs| fs.iter().find(|f| f.id == face));
            let mut v = match (found, ctx.doc.shape(body)) {
                (Some(f), Some(shape)) => face_entry(project, shape, body, f),
                _ => json!({ "key": face }),
            };
            v["what"] = json!("face");
            v["body"] = json!(body);
            v["use"] = json!({ "body": { "body": body }, "face": { "ids": [face] } });
            v
        }
        Picked::Edge { body, edge } => {
            let mut v = edge_of(project, body, edge).unwrap_or_else(|| json!({ "key": edge }));
            v["what"] = json!("edge");
            v["body"] = json!(body);
            v["use"] = json!({ "body": { "body": body }, "edges": { "ids": [edge] } });
            v
        }
        Picked::Corner { body, edge, end } => {
            let at = edge_of(project, body, edge).map(|e| if end == End::A { e["a"].clone() } else { e["b"].clone() });
            json!({ "what": "corner", "body": body, "edge": edge, "at": at })
        }
        Picked::Plane(id) => match project.planes.iter().find(|p| p.id == id) {
            Some(p) => json!({ "what": "plane", "key": id, "name": qymcad_i18n::name(&p.name), "origin": point(p.origin), "normal": point(p.normal), "use": { "plane": { "datum": id } } }),
            None => gone("plane", id),
        },
        Picked::Axis(id) => match project.datum_axes.iter().find(|a| a.id == id) {
            Some(a) => json!({ "what": "axis", "key": id, "name": qymcad_i18n::name(&a.name), "origin": point(a.origin()), "dir": point(a.dir()) }),
            None => gone("axis", id),
        },
        Picked::Point(id) => match project.datum_points.iter().find(|p| p.id == id) {
            Some(p) => json!({ "what": "point", "key": id, "name": qymcad_i18n::name(&p.name), "at": point(p.at) }),
            None => gone("point", id),
        },
        Picked::Sketch(id) => match project.sketches.iter().find(|s| s.id == id) {
            Some(s) => {
                let closed = s.contour_ids.iter().filter(|c| project.contour_profile_xy(**c).is_some()).count();
                json!({ "what": "sketch", "key": id, "name": qymcad_i18n::name(&s.name), "contours": s.contour_ids.len(), "closed": closed, "use": { "sketch": id } })
            }
            None => gone("sketch", id),
        },
        Picked::Contour(id) => {
            let sketch = project.sketches.iter().find(|s| s.contour_ids.contains(&id)).map(|s| s.id);
            json!({ "what": "contour", "key": id, "sketch": sketch, "closed": project.contour_profile_xy(id).is_some() })
        }
        Picked::Feature(id) => match project.timeline.iter().find(|n| n.id == id) {
            Some(n) => json!({
                "what": "feature", "key": id, "name": qymcad_i18n::name(&n.name), "kind": qymcad_doc::report::kind_of(&n.kind),
                "sizes": crate::tools::timeline::sizes(project, id), "use": { "feature": id },
            }),
            None => gone("feature", id),
        },
        Picked::Part(id) => match project.components.iter().find(|c| c.id == id) {
            Some(c) => json!({ "what": "part", "key": id, "name": qymcad_i18n::name(&c.name), "assembly": !project.component_is_part(id), "use": { "part": id } }),
            None => gone("part", id),
        },
        Picked::Body(id) => match project.bodies.iter().find(|b| b.id == id) {
            Some(b) => json!({ "what": "body", "key": id, "name": qymcad_i18n::name(&b.name), "part": project.body_owner(id), "use": { "body": { "body": id } } }),
            None => gone("body", id),
        },
        Picked::Joint(id) => match project.joints.iter().find(|j| j.id == id) {
            Some(j) => json!({ "what": "joint", "key": id, "name": qymcad_i18n::name(&j.name), "kind": format!("{:?}", j.kind) }),
            None => gone("joint", id),
        },
    }
}

fn edge_of(project: &Project, body: Id, edge: u32) -> Option<Value> {
    project.edge_pool(body).iter().find(|c| c.desc == edge).map(|c| edge_entry(project, body, c))
}

/// A thing the window had selected and the document no longer holds.
fn gone(what: &str, key: Id) -> Value {
    json!({ "what": what, "key": key, "gone": true })
}

pub const GET_SELECTION: Tool = Tool {
    name: "get_selection",
    description: "What the person selected in the open QymCAD window - what \"this\" means in their request. Each item: what (face, edge, corner, plane, axis, point, sketch, contour, feature, part, body, joint), its place in world coordinates, and use: the arguments the other tools take for it (a face: {\"body\", \"face\"} for create_sketch, hole, push_face; an edge: {\"body\", \"edges\"} for fillet, chamfer; a plane: {\"plane\": {\"datum\"}}; a feature: edit_feature's key). A face and an edge carry made_by: the feature that created them, with its sizes - to change \"this rounding\", edit_feature that feature; if its size is an expression, write the change into the expression of that feature (\"r - 0.2\"), not into the parameter, which other features may read. Nothing selected: an empty list - ask the person to click what they mean.",
    schema: || json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let _: Empty = tool::args(arguments)?;
        let picked = match &ctx.seen {
            Seen::NoWindow => {
                return Err(Refusal::new("no-window", "What is selected is known only in the open QymCAD window, and this server works on a document of its own.", Stage::Window)
                    .with_hint("Ask the person to switch Claude in this window on (Settings -> General) and to restart the QymCAD server; or ask them to name what they mean."));
            }
            Seen::Window(w) => w.picked.clone(),
        };
        let listed: Vec<Value> = picked.iter().map(|p| describe(ctx, p)).collect();
        let mut out = Answer::new();
        if listed.is_empty() {
            out.insert("hint".into(), json!("Nothing is selected in the window: ask the person to click what they mean."));
        }
        out.insert("selected".into(), json!(listed));
        Ok(out)
    },
};

/// No arguments at all.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
