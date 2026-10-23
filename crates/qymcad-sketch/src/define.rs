//! "FULLY DEFINE" ON THE SHEET: a contour clicked is taken whole, Enter defines what is taken.

use qymcad_ui_state::Notice;
use super::*;

/// A CLICK WITH THE TOOL IN HAND: a curve clicked takes the contour it belongs to - the curves joined to it end to end -
/// as the selection, lit, its preview drawn; a click on nothing says what the tool takes.
pub(crate) fn define_click(sk: &mut qymcad_ui_state::SketchCtx, rect: Rect, pos: Pos2) {
    let qymcad_ui_state::Sel::Sketch(si) = *sk.sel else { return };
    let Some(eid) = qymcad_ui_state::entity_near(&sk.pick(), rect, pos, si) else {
        *sk.status = Notice::refusal(qymcad_i18n::tr("sk-click-shape-line"));
        return;
    };
    let mut contour: Vec<Id> = sk.project.connected_entities(si, eid).into_iter().collect();
    contour.sort_unstable();
    sk.sel_sk.items = contour.into_iter().map(|id| qymcad_ui_state::SketchItem { kind: qymcad_ui_state::ItemKind::Curve, id }).collect();
    sk.sel_sk.at_middle.clear();
    *sk.status = Notice::info(qymcad_i18n::tr("sk-define-enter"));
}
