//! WHAT THE DOCUMENT HOLDS, AS NUMBERS: its parts, its timeline with what stands red, its bodies measured, its
//! parameters. The measures are the ones the window's own account of a document gives - the volume of the live body
//! where there is one and of the mesh where there is none, the area and the box of the mesh, the faces the body
//! keeps and the edges of the live body - so a reader of either sees one document.
//!
//! The names are left to the caller's `shown`, which knows the language; a red feature carries its error itself,
//! for the caller to turn into a code and words.
use qymcad_core::errors::CoreError;
use qymcad_core::feature::ComponentKind;
use qymcad_core::geom::Mesh;
use qymcad_core::model::{Id, Project};
use qymcad_kernel::Shape;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// The parts and assemblies of the tree, in the order they were made; the root assembly is not one of them.
    pub parts: Vec<Part>,
    /// The timeline, in its order: sketches, planes and features alike.
    pub features: Vec<Feature>,
    pub bodies: Vec<Body>,
    pub parameters: Vec<Parameter>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub key: Id,
    pub name: String,
    pub assembly: bool,
    /// The assembly it stands in, when that is not the root.
    pub parent: Option<Id>,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub key: Id,
    pub name: String,
    /// The kind of node, as the program names it in code: `Extrude`, `Fillet`, `Sketch`.
    pub kind: String,
    /// The part it belongs to.
    pub part: Option<Id>,
    pub suppressed: bool,
    /// Why it stands red after the last rebuild.
    pub error: Option<CoreError>,
    /// What it built short of what was asked.
    pub warning: Option<CoreError>,
    /// How many bodies it makes.
    pub bodies: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub id: Id,
    pub name: String,
    /// The part it belongs to.
    pub part: Option<Id>,
    /// mm^3: of the live body, or of the mesh when the program holds no live body for it.
    pub volume: f64,
    /// mm^2, of the mesh.
    pub area: f64,
    /// The box of the mesh; NaN when the mesh is empty.
    pub min: [f64; 3],
    pub max: [f64; 3],
    /// The faces the body keeps for references.
    pub faces: usize,
    /// The edges of the live body; none without one.
    pub edges: Option<usize>,
    pub visible: bool,
    /// Taken into another body by a boolean or a cut: it stands in the document, not on its own.
    pub consumed: bool,
    /// A surface with no inside.
    pub sheet: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub expr: String,
    pub value: f64,
}

/// The account of `project` with the live bodies in `shapes`; `shown` is how a stored name reads to a person.
pub fn report(project: &Project, shapes: &HashMap<Id, Shape>, shown: &dyn Fn(&str) -> String) -> Report {
    let p = project;
    let parts = p
        .components
        .iter()
        .filter(|c| c.id != p.root)
        .map(|c| Part { key: c.id, name: shown(&c.name), assembly: c.kind == ComponentKind::Assembly, parent: c.parent.filter(|id| *id != p.root), visible: c.visible })
        .collect();
    let features = p
        .timeline
        .iter()
        .map(|n| Feature {
            key: n.id,
            name: shown(&n.name),
            kind: format!("{:?}", n.kind).split([' ', '{', '(']).next().unwrap_or_default().to_string(),
            part: n.parent,
            suppressed: n.suppressed,
            error: p.regen_errors.get(&n.id).cloned(),
            warning: p.regen_warnings.get(&n.id).cloned(),
            bodies: n.kind.bodies().len(),
        })
        .collect();
    let consumed = p.consumed_bodies();
    let bodies = p
        .bodies
        .iter()
        .map(|b| {
            let shape = shapes.get(&b.id);
            let bounds = b.mesh.bounds();
            Body {
                id: b.id,
                name: shown(&b.name),
                part: p.body_owner(b.id),
                volume: shape.map_or_else(|| b.mesh.volume(), |s| s.volume()),
                area: mesh_area(&b.mesh),
                min: bounds.as_ref().map_or([f64::NAN; 3], |bb| [bb.min.x, bb.min.y, bb.min.z]),
                max: bounds.as_ref().map_or([f64::NAN; 3], |bb| [bb.max.x, bb.max.y, bb.max.z]),
                faces: b.faces.len(),
                edges: shape.map(|s| s.edges().len()),
                visible: b.visible,
                consumed: consumed.contains(&b.id),
                sheet: b.sheet,
            }
        })
        .collect();
    let parameters = p.parameters.iter().map(|q| Parameter { name: q.name.clone(), expr: q.expr.clone(), value: q.value }).collect();
    Report { parts, features, bodies, parameters }
}

/// The area of a mesh, mm^2: the sum of its triangles.
pub fn mesh_area(m: &Mesh) -> f64 {
    (0..m.tris.len())
        .map(|ti| {
            let t = m.triangle(ti);
            let u = [t[1].x - t[0].x, t[1].y - t[0].y, t[1].z - t[0].z];
            let v = [t[2].x - t[0].x, t[2].y - t[0].y, t[2].z - t[0].z];
            0.5 * ((u[1] * v[2] - u[2] * v[1]).powi(2) + (u[2] * v[0] - u[0] * v[2]).powi(2) + (u[0] * v[1] - u[1] * v[0]).powi(2)).sqrt()
        })
        .sum()
}
