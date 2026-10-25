//! A FLAT FACE CLICKED WITH "RECOGNISE FEATURE" IS REFUSED BY THE TOOL: the top of a block is no hole, rounding or
//! chamfer, and the plate of the tool says so, not the status line alone.
//!
//! Reported behaviour: the words "There is no hole, rounding or chamfer here" stood on the status line at the foot of the
//! window, and the plate of the tool by the view stayed empty.
use crate::gui::hand::Hand;
use crate::gui::App;

#[test]
fn the_top_of_a_block_is_refused_by_the_tool() {
    let mut app = App { project: qymcad_core::model::Project::default(), ..Default::default() };
    app.project.new_document();
    let _ = app.project.add_box(40.0, 30.0, 10.0);
    crate::gui::io_jobs::regenerate_now(&mut app.rebuild_ctx());
    let part = app.project.components.iter().find(|c| c.parent.is_some()).map(|c| crate::i18n::name(&c.name)).expect("the part of the block");
    let mut h = Hand::new(&mut app);
    assert!(h.double_click_word(&part), "the row of the part {part:?} is not in the tree");
    h.look_at([20.0, 15.0, 5.0], 8.0).frame(Vec::new());
    h.tool(qymcad_ui_state::PartTool::RecogniseFeature).click([20.0, 15.0, 10.0]).frame(Vec::new());
    let words = h.app.status.words.clone();
    let unfound = ["msg-unfound-no-surface", "msg-unfound-no-entry", "msg-unfound-not-two-sides", "msg-unfound-uneven", "msg-unfound-other"].map(crate::i18n::tr);
    assert!(unfound.contains(&words), "setup: the top of the block is not refused: {words}");
    let said = h.tool_says();
    assert!(said.iter().any(|n| n.words == words && n.tone == qymcad_ui_state::Tone::Refusal), "the tool does not say {words:?}: {said:?}");
}
