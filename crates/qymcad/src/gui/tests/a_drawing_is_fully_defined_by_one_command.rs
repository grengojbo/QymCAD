//! "FULLY DEFINE" IN THE WINDOW: a contour clicked, or the curves selected before the button, the sizes it would lay
//! shown dashed, and Enter lays the relations and the sizes that leave the drawing no freedom - measured from the origin,
//! one step of undo, the tool left in hand for the next contour.
use crate::gui::hand::Hand;
use crate::gui::{App, Sel};
use qymcad_core::feature::SketchPlane;

/// A window with a sketch open and the selection tool in hand, and the sketch.
struct Drawn {
    app: App,
    si: usize,
}

/// A rectangle of 40 x 30 drawn by the rectangle tool from (5, 5).
fn a_rectangle() -> Drawn {
    let mut app = App::default();
    let si = app.create_sketch_on(SketchPlane::default());
    app.chosen.sel = Sel::Sketch(si);
    Hand::new(&mut app).sk_tool(2).click2d(5.0, 5.0).click2d(45.0, 35.0).key(egui::Key::Enter);
    Hand::new(&mut app).sk_tool(0);
    Drawn { app, si }
}

fn sizes(app: &App, si: usize) -> usize {
    app.project.sketches[si].constraints.iter().filter(|c| c.dim_value().is_some()).count()
}

#[test]
fn a_contour_clicked_and_enter_leave_it_no_freedom() {
    let Drawn { mut app, si } = a_rectangle();
    let free_before = app.project.sketch_dof(si).0;
    assert!(free_before > 0, "the rectangle drawn has freedom to lose: {free_before}");
    let sizes_before = sizes(&app, si);
    let mut hand = Hand::new(&mut app);
    hand.hover2d(25.0, 20.0);
    assert!(hand.press_hint(&crate::i18n::tr("tb-fully-define-hint")), "the button of Fully define");
    hand.click2d(25.0, 5.0);
    hand.frame(Vec::new());
    // before Enter: the sizes it would lay are shown, nothing is laid yet
    let said = crate::i18n::tr("sk-define-would").split('{').next().unwrap_or_default().trim().to_string();
    assert!(hand.texts().iter().any(|t| t.contains(&said)), "the bar does not say what Enter would lay: {:?}", hand.texts());
    assert!(hand.written_at("40.0").is_some() && hand.written_at("30.0").is_some(), "the sizes it would lay are not shown on the sheet");
    assert_eq!(sizes(hand.app, si), sizes_before, "a size was laid before Enter");
    hand.key(egui::Key::Enter);
    assert_eq!(hand.app.project.sketch_dof(si).0, 0, "the contour is left free after Enter");
    assert!(sizes(hand.app, si) > sizes_before, "no size was laid");
    assert!(hand.app.tools.picking.fully_define(), "the tool is put down after one contour");
    hand.undo();
    assert_eq!(hand.app.project.sketch_dof(si).0, free_before, "one step of undo does not take the whole of it back");
    assert_eq!(sizes(hand.app, si), sizes_before, "a size stayed after undo");
}

#[test]
fn the_curves_selected_before_the_button_are_taken_at_once() {
    let Drawn { mut app, si } = a_rectangle();
    let lines: Vec<_> = app.project.sketches[si].entities.iter().map(|e| (1, e.id)).collect();
    let mut hand = Hand::new(&mut app);
    assert!(hand.select2d(&lines), "the four sides are picked");
    hand.hover2d(25.0, 20.0);
    assert!(hand.press_hint(&crate::i18n::tr("tb-fully-define-hint")), "the button of Fully define");
    hand.frame(Vec::new());
    assert!(hand.written_at("40.0").is_some(), "the sizes it would lay are not shown for the selection");
    hand.key(egui::Key::Enter);
    assert_eq!(hand.app.project.sketch_dof(si).0, 0, "the selection is left free after Enter");
}

#[test]
fn with_nothing_chosen_the_tool_waits_and_esc_puts_it_down() {
    let Drawn { mut app, si } = a_rectangle();
    let free_before = app.project.sketch_dof(si).0;
    let mut hand = Hand::new(&mut app);
    hand.hover2d(25.0, 20.0);
    assert!(hand.press_hint(&crate::i18n::tr("tb-fully-define-hint")), "the button of Fully define");
    hand.key(egui::Key::Enter);
    assert_eq!(hand.app.project.sketch_dof(si).0, free_before, "Enter with nothing taken laid something");
    assert!(hand.app.tools.picking.fully_define(), "the tool is not waiting for a contour");
    hand.key(egui::Key::Escape);
    assert!(!hand.app.tools.picking.fully_define(), "Esc does not put the tool down");
}
