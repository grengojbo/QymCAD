//! THE LIVE BODIES OF A DOCUMENT THAT HAS JUST COME IN: from the bodies written into its file, from the original
//! files of its imports kept inside it, and from its recipe where neither has one.
//!
//! A file carries the geometry to show - meshes and faces - and, where it was saved with them, the live bodies as
//! B-rep. An import has no recipe: its live body comes only from parsing its source again. What has neither is
//! rebuilt. Each of these is a step of opening a document, and each is the same with a window and without one.
use qymcad_core::model::{Id, Project};
use qymcad_kernel::Shape;
use std::collections::HashMap;

/// The live bodies written into a file as B-rep, with the names they were saved with. A blob that does not parse is
/// left out: its body is rebuilt from its recipe instead.
pub fn shapes_from_blobs(breps: Vec<(Id, Vec<u8>)>) -> Vec<(Id, Shape)> {
    let saved = breps.into_iter().map(|(body, bytes)| qymcad_kernel::imports::SavedBody { body, bytes }).collect();
    qymcad_kernel::imports::restore(saved).into_iter().map(|r| (r.body, r.shape)).collect()
}

/// What taking a loaded document in found.
pub struct Adopted {
    /// The bodies the timeline makes that the file carries no geometry for: their nodes are marked to be rebuilt.
    pub missing: Vec<Id>,
    /// The document holds imports, whose live bodies come only from their sources.
    pub imports: bool,
}

/// TAKE IN A DOCUMENT JUST READ FROM A FILE, with the live bodies written into it.
///
/// The live bodies go into the cache rather than being rebuilt: a rebuild of the whole timeline on opening cost 31 s
/// of tessellation on a document of 1170 imported solids. The faces came inside the bodies, and they go back into
/// `regen_faces`, where sketches and features resolve faces by id. Only what the file has no geometry for is marked
/// to be rebuilt.
pub fn adopt_loaded(project: &mut Project, shapes: &mut HashMap<Id, Shape>, from_file: Vec<(Id, Shape)>) -> Adopted {
    shapes.extend(from_file);
    for i in 0..project.bodies.len() {
        if let (Some(body), false) = (project.mesh_id(i), project.bodies[i].faces.is_empty()) {
            let faces = project.bodies[i].faces.clone();
            project.regen_faces.insert(body, faces);
        }
    }
    let missing: Vec<Id> = project.timeline.iter().filter_map(|n| n.kind.body()).filter(|b| project.mesh_index(*b).is_none()).collect();
    for n in &mut project.timeline {
        if n.kind.body().is_some_and(|b| missing.contains(&b)) {
            n.dirty = true;
        }
    }
    let imports = project.timeline.iter().any(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::Import { .. }));
    Adopted { missing, imports }
}

/// THE NODES WAITING FOR A LIVE BODY THEY DO NOT HAVE: a body shown from its file's mesh, with nothing exact behind it.
/// A piece of a mesh never has one and is not counted (`FeatureKind::waits_for_brep`).
pub fn missing_brep(project: &Project, shapes: &HashMap<Id, Shape>) -> Vec<Id> {
    project.timeline.iter().filter(|n| n.kind.waits_for_brep()).filter_map(|n| n.kind.body().map(|b| (n.id, b))).filter(|(_, b)| !shapes.contains_key(b)).map(|(id, _)| id).collect()
}

/// BRING THE LIVE BODIES UP where an operation needs them: only the nodes whose bodies have none are marked, so the
/// next rebuild builds them and nothing else - a forced rebuild here would cost a full tessellation of every import.
/// Returns how many were marked.
pub fn mark_missing_brep(project: &mut Project, shapes: &HashMap<Id, Shape>) -> usize {
    let missing = missing_brep(project, shapes);
    for n in &mut project.timeline {
        if missing.contains(&n.id) {
            n.dirty = true;
        }
    }
    missing.len()
}

/// FACES FOUND ON THE MESH, only for a body with no live B-rep and no faces - a raw imported STL. A body built by the
/// timeline or imported from STEP has its faces from the B-rep topology; detection on the mesh applies only where
/// there is no B-rep at all. Returns the faces given, by body, for the caller's own caches.
pub fn detect_missing_faces(project: &mut Project, shapes: &HashMap<Id, Shape>) -> Vec<(Id, Vec<qymcad_core::geom::MeshFace>)> {
    let mut given = Vec::new();
    for i in 0..project.bodies.len() {
        let has_brep = project.mesh_id(i).is_some_and(|id| shapes.contains_key(&id));
        if !has_brep && project.bodies[i].faces.is_empty() {
            let f = qymcad_meshfit::raw_faces(&project.bodies[i].mesh);
            project.bodies[i].faces = f.clone();
            if let Some(body) = project.mesh_id(i) {
                given.push((body, f));
            }
        }
    }
    given
}
