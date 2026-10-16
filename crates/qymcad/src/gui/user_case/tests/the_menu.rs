use super::*;
use crate::gui::hand::Hand;

/// UNDO AND REDO FROM THE MENU BAR, in the middle of the work: the step named in the Edit menu goes and comes back, and
/// the chain after it carries on from the restored document.
pub(super) fn the_last_step_goes_and_comes_back_from_the_menu(app: &mut App, problems: &mut Vec<String>) {
    let steps = app.disk.edits.undo.len();
    let named = app.disk.edits.undo.last().map(|s| s.name.clone()).unwrap_or_default();
    let at_menu = egui::pos2(0.0, 0.0);
    let mut hand = Hand::new(app);
    let undone = hand.press_word(&crate::i18n::tr("menu-edit"), at_menu) && hand.press_word(&crate::i18n::tr1("menu-undo-named", "what", &named), at_menu);
    if !undone || app.disk.edits.undo.len() + 1 != steps {
        problems.push(format!("[Edit -> Undo] the menu did not take back \"{named}\": {} steps, {steps} before", app.disk.edits.undo.len()));
    }
    check_all(app, "the cut was undone from the Edit menu", problems);
    let mut hand = Hand::new(app);
    let redone = hand.press_word(&crate::i18n::tr("menu-edit"), at_menu) && hand.press_word(&crate::i18n::tr1("menu-redo-named", "what", &named), at_menu);
    if !redone || app.disk.edits.undo.len() != steps {
        problems.push(format!("[Edit -> Redo] the menu did not bring back \"{named}\": {} steps, {steps} before", app.disk.edits.undo.len()));
    }
    check_all(app, "the cut was redone from the Edit menu", problems);
}
