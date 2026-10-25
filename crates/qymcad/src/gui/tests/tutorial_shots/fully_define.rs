//! The pictures of the check of "Fully define": a rectangle drawn from about (10, 10) to (50, 40), defined by a click on
//! a side; a circle beside it; a second rectangle below the axis whose sides are picked before the button.
use super::*;
use crate::gui::Sel;

#[test]
#[ignore = "draws the pictures of a manual check; run by hand"]
fn tutorial_shots_fully_define() {
    let mut shots = Shots::new("22-fully-define");
    let mut app = window();
    let si = app.create_sketch_on(qymcad_core::feature::SketchPlane::default());
    app.chosen.sel = Sel::Sketch(si);
    let mut h = Hand::new(&mut app);
    h.sk_tool(qymcad_ui_state::SketchShape::Rect).click2d(10.0, 10.0).click2d(50.0, 40.0).key(egui::Key::Enter);
    h.sk_select();
    h.hover2d(30.0, 25.0);
    take_tool(&mut h, &mut shots, "tb-fully-define-hint", Some("button"));
    shots.take(&mut h, "waiting", None);
    let side = h.on_screen2d((30.0, 10.0));
    h.click2d(30.0, 10.0);
    shots.take(&mut h, "rectangle-preview", Some(side));
    h.key(egui::Key::Enter);
    shots.take(&mut h, "rectangle-laid", None);
    h.key(egui::Key::Escape);

    h.sk_tool(qymcad_ui_state::SketchShape::Circle).click2d(75.0, 25.0).click2d(82.0, 25.0).key(egui::Key::Escape);
    h.sk_select();
    take_tool(&mut h, &mut shots, "tb-fully-define-hint", None);
    let rim = h.on_screen2d((75.0, 32.0));
    h.click2d(75.0, 32.0);
    shots.take(&mut h, "circle-preview", Some(rim));
    h.key(egui::Key::Enter);
    h.key(egui::Key::Escape);

    // below the axis, where the view drawn for the first shows it whole
    h.sk_tool(qymcad_ui_state::SketchShape::Rect).click2d(10.0, -50.0).click2d(40.0, -30.0).key(egui::Key::Enter);
    h.sk_select();
    h.click2d(25.0, -50.0);
    h.shift_click2d(40.0, -40.0);
    h.shift_click2d(25.0, -30.0);
    shots.take(&mut h, "sides-picked", None);
    take_tool(&mut h, &mut shots, "tb-fully-define-hint", None);
    shots.take(&mut h, "picked-preview", None);
    h.key(egui::Key::Enter);
    // taken before Esc puts the tool down: the status line then tells what was laid, not that the tool was let go
    shots.take(&mut h, "all-laid", None);
    h.key(egui::Key::Escape);
}
