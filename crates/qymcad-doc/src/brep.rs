//! THE LIVE BODIES OF A DOCUMENT THAT HAS JUST COME IN: from the bodies written into its file, from the original
//! files of its imports kept inside it, and from its recipe where neither has one.
//!
//! A file carries the geometry to show - meshes and faces - and, where it was saved with them, the live bodies as
//! B-rep. An import has no recipe: its live body comes only from parsing its source again. What has neither is
//! rebuilt. Each of these is a step of opening a document, and each is the same with a window and without one.
use qymcad_core::model::{Id, Project};
use qymcad_kernel::Shape;
use std::collections::HashMap;

/// A uniform scale about the origin as a 3x4 row-major matrix, the form the kernel takes a placement in.
pub fn scale_matrix(f: f64) -> [f64; 12] {
    [f, 0.0, 0.0, 0.0, 0.0, f, 0.0, 0.0, 0.0, 0.0, f, 0.0]
}

/// The live bodies written into a file as B-rep, parsed under the kernel's lock. A blob that does not parse is left
/// out: its body is rebuilt from its recipe instead.
pub fn shapes_from_blobs(breps: Vec<(Id, Vec<u8>)>) -> Vec<(Id, Shape)> {
    let _gate = qymcad_kernel::kernel_gate();
    breps.into_iter().filter_map(|(id, b)| Shape::from_brep_bytes(&b).map(|s| (id, s))).collect()
}

/// THE LIVE BODIES OF IMPORTS, raised again from the original files kept inside the document (`sources/`).
///
/// Each source is parsed once and its solids are laid out across the bodies that came from it, each by its index in
/// the file. A source is read by what it is - an IGES source read as STEP gave nothing, and the part opened with no
/// live body - and every solid is put back at the factor it was taken at when the file came in, because a file can
/// name the wrong unit: without the factor a 10 mm cube taken at 25.4 comes back at 1000 mm^3 instead of
/// 16 387 064 mm^3.
pub fn import_shapes(project: &Project) -> Vec<(Id, Shape)> {
    use qymcad_core::feature::FeatureKind;
    // group by source, so that each file is unpacked only once
    let mut by_src: HashMap<Id, Vec<(Id, u32, f64)>> = HashMap::new();
    for n in &project.timeline {
        if let FeatureKind::Import { body, source, solid, scale } = n.kind {
            by_src.entry(source).or_default().push((body, solid, scale));
        }
    }
    let mut out = Vec::new();
    for (src, items) in by_src {
        let Some(sf) = project.sources.iter().find(|s| s.id == src) else { continue };
        if sf.data.is_empty() {
            continue;
        }
        let ext = if sf.ext.is_empty() { "step" } else { sf.ext.as_str() };
        // a name of its own for every parse: two documents read at once in one process may share a source id
        static PARSES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let k = PARSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = std::env::temp_dir().join(format!("qym_import_{}_{k}_{src}.{ext}", std::process::id()));
        if std::fs::write(&tmp, &sf.data).is_err() {
            continue;
        }
        let format = qymcad_kernel::ExactFormat::of_extension(ext);
        // the Option wrapper: a shape is moved (it is not Clone), and each solid is taken by index exactly once
        let mut shapes: Vec<Option<Shape>> = qymcad_kernel::exact_solids(format, tmp.to_string_lossy().as_ref()).unwrap_or_default().into_iter().map(Some).collect();
        let _ = std::fs::remove_file(&tmp);
        for (body, solid, scale) in items {
            if let Some(s) = shapes.get_mut(solid as usize).and_then(|o| o.take()) {
                let s = if (scale - 1.0).abs() > 1e-12 { s.transformed(&scale_matrix(scale)).unwrap_or(s) } else { s };
                out.push((body, s));
            }
        }
    }
    out
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
            let f = project.bodies[i].mesh.detect_faces(8.0);
            project.bodies[i].faces = f.clone();
            if let Some(body) = project.mesh_id(i) {
                given.push((body, f));
            }
        }
    }
    given
}
