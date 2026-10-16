//! "FULLY DEFINE" IN A SKETCH: the curves taken, the relations and the sizes that leave them no freedom shown before
//! Enter, and laid by it as one step of undo - measured from the origin of the sketch.

use crate::Notice;
use super::*;
use qymcad_core::model::{Anchor, Defined};
use std::cell::RefCell;
use std::rc::Rc;

/// THE TOOL TAKEN: held either way. The curves selected before it are what it defines and its preview is up at once;
/// with none it waits for a contour clicked.
pub fn start_fully_define(picking: &mut Picking, sel: Sel, sel_sk: &SketchSelection, status: &mut Notice) {
    if !matches!(sel, Sel::Sketch(_)) {
        return;
    }
    *picking = Picking::FullyDefine;
    *status = Notice::info(qymcad_i18n::tr(if define_curves(sel_sk).is_empty() { "g-define-hint" } else { "sk-define-enter" }));
}

/// THE CURVES THE TOOL DEFINES: the lines, arcs and circles selected.
pub fn define_curves(sel_sk: &SketchSelection) -> Vec<Id> {
    sel_sk.items.iter().filter(|(k, _)| *k == 1).map(|(_, id)| *id).collect()
}

/// WHAT ENTER WOULD LAY: the document as it would stand after it, the constraints laid in the sketch from `from` on,
/// and what was counted.
pub struct DefinePreview {
    pub project: Project,
    pub si: usize,
    pub from: usize,
    pub defined: Defined,
}

/// What a preview was worked out for: the sketch, the curves, where its points stood and how many constraints it held.
/// A drag, a size typed or a constraint laid changes one of them and the preview is worked out again.
#[derive(PartialEq)]
struct DefineKey {
    si: usize,
    curves: Vec<Id>,
    places: Vec<u64>,
    constraints: usize,
}

/// A preview and what it was worked out for.
struct Remembered {
    key: DefineKey,
    preview: Rc<DefinePreview>,
}

thread_local! {
    // A PREVIEW IS REMEMBERED BETWEEN FRAMES: each size it offers is laid and the sketch solved to judge it, which is
    // too much for every frame of a window that stands still.
    static PREVIEW: RefCell<Option<Remembered>> = const { RefCell::new(None) };
}

fn key_of(project: &Project, si: usize, curves: &[Id]) -> Option<DefineKey> {
    let s = project.sketches.get(si)?;
    let mut curves = curves.to_vec();
    curves.sort_unstable();
    let places = s.points.iter().flat_map(|p| [p.x.to_bits(), p.y.to_bits()]).collect();
    Some(DefineKey { si, curves, places, constraints: s.constraints.len() })
}

/// THE PREVIEW OF `curves` OF SKETCH `si` fully defined: worked out on a copy of the document, remembered while the
/// sketch stands as it was. None with no curve taken.
pub fn define_preview(project: &Project, si: usize, curves: &[Id]) -> Option<Rc<DefinePreview>> {
    if curves.is_empty() {
        return None;
    }
    let key = key_of(project, si, curves)?;
    if let Some(found) = PREVIEW.with(|p| p.borrow().as_ref().filter(|r| r.key == key).map(|r| r.preview.clone())) {
        return Some(found);
    }
    let mut copy = project.clone();
    let from = copy.sketches[si].constraints.len();
    let defined = copy.fully_define(si, curves, &Anchor::Origin);
    let preview = Rc::new(DefinePreview { project: copy, si, from, defined });
    PREVIEW.with(|p| *p.borrow_mut() = Some(Remembered { key, preview: preview.clone() }));
    Some(preview)
}

/// ENTER: the curves selected fully defined, one step of undo, the selection let go and what was laid said. The tool
/// stays in hand for the next contour. False with no curve selected.
pub fn fully_define_now(ed: Editing, sel_sk: &mut SketchSelection) -> bool {
    let Sel::Sketch(si) = *ed.sel else { return false };
    let curves = define_curves(sel_sk);
    if curves.is_empty() {
        return false;
    }
    begin_edit(ed.edits, &*ed.project, qymcad_i18n::tr("tool-fully-define"));
    let defined = ed.project.fully_define(si, &curves, &Anchor::Origin);
    close_edit(ed.edits, ed.project);
    invalidate(ed.regen);
    sel_sk.items.clear();
    sel_sk.at_middle.clear();
    let said = told_defined(&defined, "msg-define-laid");
    *ed.status = if defined.free == 0 { Notice::info(said) } else { Notice::warning(said) };
    true
}

/// WHAT A DEFINITION LAID, in words: its relations and sizes after the words of `key`, whether the drawing is fully
/// defined, and each size rounded - "29.97 -> 30.0".
pub fn told_defined(d: &Defined, key: &str) -> String {
    let state = if d.free == 0 { qymcad_i18n::tr("dof-fully-defined") } else { qymcad_i18n::tr1("sk-dof-n", "n", &d.free.to_string()) };
    let mut out = qymcad_i18n::trn(key, &[("rel", &d.relations.to_string()), ("sizes", &d.sizes.to_string()), ("state", &state)]);
    if !d.rounded.is_empty() {
        let list: Vec<String> = d.rounded.iter().map(|r| format!("{} -> {}", qymcad_i18n::num(r.was, 2), qymcad_i18n::num(r.now, 1))).collect();
        out = format!("{out}; {}", qymcad_i18n::tr1("msg-sketch-from-face-rounded", "list", &list.join(", ")));
    }
    out
}
