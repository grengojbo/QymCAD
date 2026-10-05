//! A PARAMETER CHANGED: what follows in the document before the rebuild.
//!
//! The values are evaluated again, and every sketch with a dimension written as an expression is solved again and
//! marked for the rebuild. The features whose own expressions read a changed name are marked by the rebuild itself,
//! against the values it was last made with, so nothing of that is repeated here.

use qymcad_core::model::Project;

/// Bring the document in line with its parameters after one of them was added, changed or deleted.
pub fn settle(project: &mut Project) {
    let _ = project.eval_parameters();
    // THE REBUILD GRAPH: editing a parameter touches only the sketches that actually mention one, not every one
    // of them.
    for si in 0..project.sketches.len() {
        if project.sketches[si].constraints.iter().any(|c| c.expr().is_some()) {
            project.solve_sketch(si);
            let sid = project.sketches[si].id;
            project.mark_sketch_dirty(sid);
        }
    }
}
