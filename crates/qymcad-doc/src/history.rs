//! UNDO AND REDO: an action is one step, a step is a snapshot of the document before it, and taking a step back puts
//! that snapshot in place of the document - with everything a snapshot does not carry brought back from the live
//! state, and exactly the bodies whose recipe differs marked to be rebuilt.
//!
//! A snapshot carries the meshes but not the live bodies (a `Shape` does not clone) and not the bytes of the
//! original files imported once (tens of megabytes that never change). Both are put back here; the words of the
//! window - the status line, the selection, the view - are the caller's.
use qymcad_core::model::{Id, Project};
use qymcad_kernel::Shape;
use std::collections::HashMap;

/// A SNAPSHOT OF THE DOCUMENT for the history.
///
/// THE BYTES OF THE EMBEDDED SOURCES DO NOT GO INTO IT. They never change - they are the original of a file imported
/// once - and they weigh tens of megabytes: on a real assembly 89 MB times 40 undo steps is gigabytes of memory and
/// about 90 MB of memcpy for every committed edit. Restoring puts them back from the live document by id. The faces
/// and edges of the rebuild are derived, and the snapshot holds the faces in the bodies themselves.
pub fn snapshot(project: &Project) -> Project {
    let mut p = project.clone_without_source_data();
    p.regen_faces.clear();
    p.regen_edges.clear();
    p
}

/// The key of a document's state: two documents with one key hold the same model. Built from the core's own
/// hand-written key, without a serialisation of the document.
pub fn doc_key(project: &Project) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    project.state_key().hash(&mut h);
    h.finish()
}

/// WHAT AN UNDO TOOK OUT OF THE DOCUMENT, kept for the redo that brings it back: the live bodies of imports, which come
/// only from parsing their file again, and the bytes of sources, without which a drawing brought back had no geometry
/// to bring in. The two maps are the caller's; this only names them together.
pub struct Shelf<'a> {
    pub shapes: &'a mut HashMap<Id, Shape>,
    pub sources: &'a mut HashMap<Id, Vec<u8>>,
}

/// What a restore changed, for the caller to finish with.
pub struct Restored {
    /// The bodies whose recipe differs between the two states: their nodes are marked, and the next rebuild brings
    /// their live bodies up to the restored state.
    pub changed: Vec<Id>,
    /// The pieces of a mesh that differ: their mesh and faces came back with the snapshot, and nothing is rebuilt.
    pub pieces: Vec<Id>,
}

/// PUT A SNAPSHOT IN PLACE OF THE DOCUMENT.
///
/// What is rebuilt is exactly the bodies whose RECIPE differs between the states, not a forced pass over the whole
/// document (on an assembly of a thousand imports that is tens of seconds). Left as they were, the live bodies of the
/// undone state stayed in the kernel: old geometry on screen, new geometry underneath, and the NEXT operation built
/// on the undone shape - silently, because nodes are not dirty after an undo.
///
/// THE LIVE BODIES OF THE CHANGED ONES STAY until the rebuild replaces them: every node of theirs is marked and built
/// again, and one that no longer builds keeps its last good body, as a partial rebuild keeps it. Reported behaviour:
/// a sketch deleted under its extrusion left the body with 12 edges to pick; the same step undone and done again had
/// none.
pub fn restore_snapshot(project: &mut Project, shapes: &mut HashMap<Id, Shape>, shelf: Shelf, snap: Project) -> Restored {
    let mut changed = project.changed_bodies_vs(&snap);
    // A PIECE OF A MESH IS ITS OWN GEOMETRY: the snapshot brings its mesh and faces back as they were, and there is
    // nothing to rebuild it from. Counted as changed, its node is marked and a rebuild of nothing is started.
    let pieces: Vec<Id> = changed.iter().copied().filter(|&b| snap.timeline.iter().any(|n| n.kind.owns_body(b) && !n.kind.waits_for_brep())).collect();
    changed.retain(|b| !pieces.contains(b));
    let mut restored = snap;
    // THE PARAMETERS THAT DIFFER between the two states change whatever reads them, though no recipe changed: the
    // node still says "k". A snapshot taken when an edit closed holds the new expression beside the old value and the
    // old geometry, and restored as it was, a chamfer driven by k kept the size of the k before. Measured by the check
    // of undo and redo after every step: k written as 3 came back holding 1.5, the chamfer with it. Asked by the
    // expression as well as the value for that reason; the values are counted again from the expressions first, as
    // the snapshot may hold an expression its value never caught up with.
    let _ = restored.eval_parameters();
    let said = |p: &Project| p.parameters.iter().map(|q| (q.name.to_lowercase(), (q.expr.clone(), q.value.to_bits()))).collect::<HashMap<_, _>>();
    let (was, now) = (said(project), said(&restored));
    let moved: Vec<String> = was.keys().chain(now.keys()).filter(|k| was.get(*k) != now.get(*k)).cloned().collect();
    shelve_source_data(shelf.sources, project, &mut restored);
    restored.take_source_data_from(project); // the source bytes come from the live document
    restored.keep_ids_past(project); // an id handed out once is never handed out again
                                     // the derived topology caches never went into the snapshot - they come back from the live state for the bodies the
                                     // edit did not touch (the changed ones are rebuilt anyway)
    let (rf, re) = (std::mem::take(&mut project.regen_faces), std::mem::take(&mut project.regen_edges));
    shelve_imports(shapes, shelf.shapes, project, &restored);
    *project = restored;
    project.regen_faces = rf;
    project.regen_edges = re;
    // every body of the node counts: a pattern of parts and a split body have several and no single `body()`, and a
    // pattern brought back by redo kept its copies without a live shape - no edges to pick on any of them
    let dirty: Vec<Id> = project.timeline.iter().filter(|n| n.kind.bodies().iter().any(|b| changed.contains(b))).map(|n| n.id).collect();
    for n in &mut project.timeline {
        if dirty.contains(&n.id) {
            n.dirty = true;
        }
    }
    for name in &moved {
        project.mark_param_dependents_dirty_for(name);
    }
    Restored { changed, pieces }
}

/// The imported bodies of `p`.
fn import_bodies(p: &Project) -> std::collections::HashSet<Id> {
    p.timeline
        .iter()
        .filter_map(|n| match n.kind {
            qymcad_core::feature::FeatureKind::Import { body, .. } => Some(body),
            _ => None,
        })
        .collect()
}

/// Between two states of the document: the live shapes of the imports leaving it go to the shelf, those of the
/// imports coming back are taken off it, so the rebuild after the restore tessellates them with named faces. An
/// import has no recipe: without its shape it came back from redo as the snapshot's mesh, faces unnamed and no edges
/// to pick.
fn shelve_imports(shapes: &mut HashMap<Id, Shape>, shelf: &mut HashMap<Id, Shape>, was: &Project, now: &Project) {
    let (before, after) = (import_bodies(was), import_bodies(now));
    for b in before.difference(&after) {
        if let Some(s) = shapes.remove(b) {
            shelf.insert(*b, s);
        }
    }
    for b in &after {
        if !shapes.contains_key(b) {
            if let Some(s) = shelf.remove(b) {
                shapes.insert(*b, s);
            }
        }
    }
}

/// Between two states of the document: the bytes of the sources leaving it go to the shelf, those of the sources
/// coming back without bytes are taken off it.
fn shelve_source_data(shelf: &mut HashMap<Id, Vec<u8>>, was: &mut Project, now: &mut Project) {
    for src in &mut was.sources {
        if !src.data.is_empty() && !now.sources.iter().any(|n| n.id == src.id) {
            shelf.insert(src.id, std::mem::take(&mut src.data));
        }
    }
    for src in &mut now.sources {
        if src.data.is_empty() {
            if let Some(d) = shelf.remove(&src.id) {
                src.data = d;
            }
        }
    }
}

/// THE HISTORY OF A DOCUMENT: the steps undo takes back and redo puts again, each a name and the state before it.
///
/// An action is opened under a name and closed by a commit (the step goes on the undo stack) or an abort (the state
/// before it is handed back to be restored, and no step is left). Nested actions merge into ONE: an operation touches
/// the sketch, the timeline and the selection, and that is one action and one undo step.
pub struct History {
    undo: Vec<(String, Project)>,
    redo: Vec<(String, Project)>,
    /// The action open now: its name and the state before it.
    open: Option<(String, Project)>,
    /// How deep the actions are nested: an inner commit must not close the outer action.
    depth: usize,
    /// How many steps are kept; the oldest goes first.
    cap: usize,
}

impl History {
    pub fn new(cap: usize) -> Self {
        History { undo: Vec::new(), redo: Vec::new(), open: None, depth: 0, cap: cap.max(1) }
    }

    /// Open an action under `name`, before it changes `project`.
    pub fn begin(&mut self, name: &str, project: &Project) {
        if self.open.is_none() {
            self.open = Some((name.to_string(), snapshot(project)));
        }
        self.depth += 1;
    }

    /// Close the action as one named step. An inner action leaves the outer one open.
    pub fn commit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth > 0 {
            return;
        }
        let Some(step) = self.open.take() else { return };
        self.undo.push(step);
        if self.undo.len() > self.cap {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Abort the action: the state before it, for the caller to restore, and no step left behind. `None` while an
    /// outer action is still open, or when none was.
    #[must_use]
    pub fn abort(&mut self) -> Option<Project> {
        self.depth = self.depth.saturating_sub(1);
        if self.depth > 0 {
            return None;
        }
        self.open.take().map(|(_, before)| before)
    }

    /// An action is open: begun and neither committed nor aborted.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// One step back from `now`: the step's name and the state to restore; `now` becomes the step redo puts again.
    /// Nothing while an action is open - it is closed first.
    #[must_use]
    pub fn undo(&mut self, now: &Project) -> Option<(String, Project)> {
        if self.open.is_some() {
            return None;
        }
        let (name, snap) = self.undo.pop()?;
        self.redo.push((name.clone(), snapshot(now)));
        Some((name, snap))
    }

    /// One step forward from `now`: the step's name and the state to restore.
    #[must_use]
    pub fn redo(&mut self, now: &Project) -> Option<(String, Project)> {
        if self.open.is_some() {
            return None;
        }
        let (name, snap) = self.redo.pop()?;
        self.undo.push((name.clone(), snapshot(now)));
        Some((name, snap))
    }

    /// The names of the steps undo would take back, oldest first.
    pub fn undo_names(&self) -> Vec<&str> {
        self.undo.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// The names of the steps redo would put again, the next one last.
    pub fn redo_names(&self) -> Vec<&str> {
        self.redo.iter().map(|(n, _)| n.as_str()).collect()
    }
}
