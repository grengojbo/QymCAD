//! The reproduction harness: a headless regeneration of a loaded project by the real kernel — the very same
//! `qymcad_kernel::OcctKernel` the application drives, shared code rather than a copy, since the copies had
//! drifted apart in their tessellation and the tests then measured something other than what appears on screen.
//! It exists to debug defects against a particular file.
#![allow(clippy::too_many_arguments, dead_code)]
use qymcad_core::model::{Id, Project};
use std::collections::HashMap;

/// A forced regeneration of the whole project by the real kernel. It returns the report and the cache of live
/// shapes by body.
pub fn regenerate(project: &mut Project) -> (qymcad_core::feature::RegenReport, HashMap<Id, qymcad_kernel::Shape>) {
    regenerate_with_shapes(project, HashMap::new())
}

/// A regeneration without forcing: only what is already marked dirty is rebuilt, which is how the application
/// behaves after a local edit such as changing a dimension or deleting a node. The kernel cache is seeded with
/// ready shapes.
///
/// It runs the rebuild of the window itself (`qymcad_doc::regen::run`): the ghosts are pruned before it, and the
/// faces of a rebuilt body go into the body after it. The parameters count as seen at their present values, so
/// nothing beyond what the caller marked is rebuilt.
pub fn regenerate_dirty_with_shapes(project: &mut Project, mut shapes: HashMap<Id, qymcad_kernel::Shape>) -> (qymcad_core::feature::RegenReport, HashMap<Id, qymcad_kernel::Shape>) {
    let mut seen = project.param_map();
    let done = qymcad_doc::regen::run(project, &mut shapes, &mut seen, None);
    (done.report, shapes)
}

/// As [`regenerate`], but with the kernel cache seeded with ready shapes, which is what the application does
/// when opening a project: imported bodies are restored from the embedded STEP before the rebuild. This is what
/// an honest measurement of opening a large file rests on.
pub fn regenerate_with_shapes(project: &mut Project, shapes: HashMap<Id, qymcad_kernel::Shape>) -> (qymcad_core::feature::RegenReport, HashMap<Id, qymcad_kernel::Shape>) {
    qymcad_doc::regen::mark_all_dirty(project);
    regenerate_dirty_with_shapes(project, shapes)
}

/// Restore the live bodies of imports from their embedded sources: the same thing the application does when
/// opening a document, through `qymcad_doc::brep::import_shapes` and `ensure_brep`. Each body comes back at the
/// factor its import node keeps.
///
/// Why it belongs here. An imported body comes from a STEP file and has no recipe, while a rebuild can only
/// re-tessellate a shape that is already live. A document opened from a bundle has no live shapes - they are
/// raised on demand - so `regen_faces` and `regen_edges` stayed empty for imports. All the derived geometry is
/// taken from those: the principal direction of a face, the axis of a cylinder, the reference direction of an
/// edge. Without them a joint on the face of an imported part has no geometric direction and falls back to the
/// world axes.
///
/// In a real assembly nearly every part is imported, so a headless check without this step measures a different
/// document from the one on screen.
pub fn restore_import_shapes(project: &Project) -> HashMap<Id, qymcad_kernel::Shape> {
    qymcad_doc::brep::import_shapes(project).into_iter().collect()
}

/// Open a document exactly as it is seen on screen: with the bodies of imports restored and a full rebuild. It
/// returns the report of the regeneration.
pub fn open_like_the_app(project: &mut Project) -> qymcad_core::feature::RegenReport {
    let shapes = restore_import_shapes(project);
    regenerate_with_shapes(project, shapes).0
}
