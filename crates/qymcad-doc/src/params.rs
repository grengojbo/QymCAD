//! A PARAMETER CHANGED: what follows in the document before the rebuild.
//!
//! The values are evaluated again, and every sketch with a dimension written as an expression is solved again, so the
//! sizes on screen follow at once. What is rebuilt is the rebuild's own to decide: it marks what reads a name whose value
//! changed, against the values it was last made with. Every sketch with a formula used to be marked here, whatever
//! changed: an empty row added to the table of parameters - no value changed - rebuilt two sketches, the cuts on them
//! and a rounding of 36 edges for 7.8 s.

use qymcad_core::model::Project;

/// Bring the document in line with its parameters after one of them was added, changed or deleted.
pub fn settle(project: &mut Project) {
    let _ = project.eval_parameters();
    for si in 0..project.sketches.len() {
        if project.sketches[si].constraints.iter().any(|c| c.expr().is_some()) {
            project.solve_sketch(si);
        }
    }
}
