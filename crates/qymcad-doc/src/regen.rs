//! THE REBUILD, as one sequence: what is made ready before the kernel runs, the run itself, and what is settled after
//! it. The window and every headless caller go through these same steps, so a body rebuilt without a screen is the
//! body the screen shows.
//!
//! The words of the window - the status line, the rebinds it announces, the caches of its own drawing - are not
//! here: the caller reads them out of the report this returns.
use qymcad_core::geom::MeshFace;
use qymcad_core::model::{Id, Project};
use qymcad_kernel::{OcctKernel, Shape};
use std::collections::HashMap;

/// What a rebuild answers: the core's report, and how many references were found by place rather than by name.
pub struct Rebuilt {
    pub report: qymcad_core::feature::RegenReport,
    /// HOW OFTEN THE GEOMETRIC FALLBACK FIRED, counted around the rebuild. This is an event of another kind than a
    /// rebind: there a name was found and moved, here no name was found and the element was taken BY PLACE, by
    /// resemblance. That is how a reference lands on a neighbouring face, so it is counted, never passed over.
    pub snaps: u32,
}

/// Everything a rebuild needs made ready before the kernel runs.
///
/// THE GHOSTS GO FIRST: a mesh no node owns any more, and a node left pointing at nothing, are removed on every
/// rebuild, and their live shapes with them.
///
/// THE REBUILD GRAPH: a parameter that changed since the last rebuild - a named sketch dimension included, those are
/// in `param_map` - marks ONLY the features that refer to it. Marking every feature that carries an expression fired
/// on every rebuild, and any trifle dragged the whole parametrics of a project through a recount.
pub fn prepare(project: &mut Project, shapes: &mut HashMap<Id, Shape>, params_seen: &HashMap<String, f64>) {
    for gone in project.prune_dangling() {
        shapes.remove(&gone);
    }
    mark_changed_params_dirty(params_seen, project);
}

/// Mark what reads a parameter whose value is not the one seen at the last rebuild, or that is gone.
pub fn mark_changed_params_dirty(params_seen: &HashMap<String, f64>, project: &mut Project) {
    let vars = project.param_map();
    let mut changed: Vec<String> = vars.iter().filter(|(k, v)| params_seen.get(*k).is_none_or(|old| (*old - **v).abs() > 1e-12)).map(|(k, _)| k.clone()).collect();
    // a name that was seen and is gone - a parameter deleted - changes whatever was counted from it
    changed.extend(params_seen.keys().filter(|k| !vars.contains_key(*k)).cloned());
    for name in &changed {
        project.mark_param_dependents_dirty_for(name);
    }
}

/// Everything settled after the kernel has run.
///
/// THE PARAMETER VALUES ARE REMEMBERED ON THE FACT of a finished rebuild, not before it: named dimensions are derived
/// from the sketch geometry, and the sketches are solved by the rebuild itself. A snapshot taken before would drift
/// from the result for no reason, and the next rebuild would take the parameter for changed again.
///
/// THE CACHE OF LIVE SHAPES MUST MATCH WHAT THE PROJECT HOLDS. A rebuild removes the mesh of a body that stopped
/// building - a rollback, a suppression, a cascade of an error - while its shape stayed in the cache as a ghost: a
/// foreign volume in the counts, wasted memory, and dead geometry handed outside. An import keeps its shape: it has
/// no recipe to build one again from.
///
/// THE FACES OF A REBUILT BODY GO INTO THE BODY, where references, the file and every measurement read them.
pub fn settle(project: &mut Project, shapes: &mut HashMap<Id, Shape>, params_seen: &mut HashMap<String, f64>, built: &[(Id, Vec<MeshFace>)]) {
    *params_seen = project.param_map();
    keep_live_shapes(shapes, project);
    for (body, faces) in built {
        if let Some(i) = project.mesh_index(*body) {
            project.bodies[i].faces = faces.clone();
        }
    }
}

/// Drop the live shapes of bodies the project no longer holds; the shapes of imports stay.
pub fn keep_live_shapes(shapes: &mut HashMap<Id, Shape>, project: &Project) {
    let imports: std::collections::HashSet<Id> = project
        .timeline
        .iter()
        .filter_map(|n| match n.kind {
            qymcad_core::feature::FeatureKind::Import { body, .. } => Some(body),
            _ => None,
        })
        .collect();
    shapes.retain(|body, _| imports.contains(body) || project.mesh_index(*body).is_some());
}

/// THE WHOLE REBUILD, here and now: made ready, run by the real kernel over the live shapes, settled.
///
/// `threads` is how many cores the kernel's booleans may take, said where the rebuild starts rather than remembered
/// somewhere: one means a single thread, zero or less all the cores but one. `None` leaves the kernel as it was set.
pub fn run(project: &mut Project, shapes: &mut HashMap<Id, Shape>, params_seen: &mut HashMap<String, f64>, threads: Option<i32>) -> Rebuilt {
    prepare(project, shapes, params_seen);
    let _gate = qymcad_kernel::kernel_gate();
    if let Some(t) = threads {
        qymcad_kernel::set_parallel(t != 1, t);
    }
    let kernel = OcctKernel { shapes: std::cell::RefCell::new(std::mem::take(shapes)), quality_k: project.geom_quality.deflection_k(), ..Default::default() };
    let snaps_before = project.snap_rebinds.load(std::sync::atomic::Ordering::Relaxed);
    let report = project.regenerate(&kernel);
    let snaps = project.snap_rebinds.load(std::sync::atomic::Ordering::Relaxed).saturating_sub(snaps_before);
    *shapes = kernel.shapes.into_inner();
    settle(project, shapes, params_seen, &report.built);
    Rebuilt { report, snaps }
}

/// Mark every node that makes a body: the next rebuild builds the whole document from its recipe, as opening a file
/// with no live shapes in it does.
pub fn mark_all_dirty(project: &mut Project) {
    for n in &mut project.timeline {
        if n.kind.body().is_some() {
            n.dirty = true;
        }
    }
}
