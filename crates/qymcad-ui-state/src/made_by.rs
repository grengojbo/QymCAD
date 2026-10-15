//! WHAT MADE A PIECE OF A BODY, AND WHAT A NODE MADE: the tie between a face in the view and its row in the tree.
//!
//! The selection is one value - a face, or a node - and each side lit only its own kind: a face clicked left the tree
//! dark, a row clicked left the body dark. Reported behaviour: "a circle selected and nothing lit in the tree - it is not
//! clear what to edit; a row selected and nothing lit on the model". The name table of the document says which node
//! made each face (`NameTable::get(face).feature`); read both ways, it lights the other side.
use qymcad_core::model::{Id, Project};

use crate::Sel;

/// A FACE OF A SHOWN MESH: the mesh by its index in `Project::bodies`, the face by its index in that mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceAt {
    pub mesh: usize,
    pub face: usize,
}

/// THE NODE THAT MADE WHAT `sel` IS: for a face the node its name records, for an edge the later of the makers of the
/// two faces it runs between - the rim of a rounding is put there by the rounding. A face with no name (a body come
/// from a file or a mesh) is answered by the node that made its body. Anything else answers `None`.
pub fn maker_of(project: &Project, sel: Sel) -> Option<Id> {
    let in_tree = |id: Id| project.timeline.iter().any(|n| n.id == id);
    match sel {
        Sel::Face(mi, fi) => {
            let face = project.bodies.get(mi)?.faces.get(fi)?;
            project.names.get(face.id).map(|n| n.feature).filter(|f| in_tree(*f)).or_else(|| project.creator_of(project.mesh_id(mi)?))
        }
        Sel::Edge(body, edge) | Sel::Vertex(body, edge, _) => {
            let place = |f: Id| project.timeline.iter().position(|n| n.id == f);
            let named = project.names.edge(edge).and_then(|e| e.faces.iter().filter_map(|&f| project.names.get(f).map(|n| n.feature)).filter(|f| place(*f).is_some()).max_by_key(|f| place(*f)));
            named.or_else(|| project.creator_of(body))
        }
        _ => None,
    }
}

/// THE FACES NODE `node` MADE, on the meshes `shown` lets through: the faces whose name records it as their maker. A
/// node that made no named face (a sketch, a body from a file) answers none.
pub fn faces_made_by(project: &Project, node: Id, shown: impl Fn(usize) -> bool) -> Vec<FaceAt> {
    let mut out = Vec::new();
    for (mesh, body) in project.bodies.iter().enumerate() {
        if !shown(mesh) {
            continue;
        }
        for (face, f) in body.faces.iter().enumerate() {
            if project.names.get(f.id).is_some_and(|n| n.feature == node) {
                out.push(FaceAt { mesh, face });
            }
        }
    }
    out
}
