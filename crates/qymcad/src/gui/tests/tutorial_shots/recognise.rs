//! The pictures of the check of "Recognise feature": a block 40 x 30 x 10 drilled 10 through at (10, 15) and 6 blind 5
//! deep at (28, 15), its upright edge at (40, 30) rounded 4 and the one at the origin cut back 2, come in as an STL file.
use super::*;

/// The block of the check.
pub(super) fn sample() -> Shape {
    let b = block([40.0, 30.0, 10.0]);
    let b = b.filleted(4.0, &[upright(&b, [40.0, 30.0])]).expect("rounded");
    let b = b.chamfered(2.0, &[upright(&b, [0.0, 0.0])]).expect("cut");
    let b = drilled(b, Drill { at: [10.0, 15.0], r: 5.0, from: -1.0, h: 12.0 });
    drilled(b, Drill { at: [28.0, 15.0], r: 3.0, from: 5.0, h: 6.0 })
}

/// The block of the check brought in as an STL file, made a polyhedron and its four features made exact, taken as the
/// check before says to.
pub(super) fn recognised(h: &mut Hand, file: &Written) {
    brought_in(h, file);
    h.look_at([20.0, 15.0, 5.0], 12.0);
    h.tool(qymcad_ui_state::PartTool::Recognise);
    h.press3d([20.0, 5.0, 10.0], egui::Modifiers::NONE);
    h.press_word(&crate::i18n::tr("cmd-recognise-asis"), egui::pos2(700.0, 60.0));
    enter(h);
    h.key(egui::Key::Escape);
    h.tool(qymcad_ui_state::PartTool::RecogniseFeature);
    for at in [
        wall_seen(h, [10.0, 15.0, 8.0], 5.0),
        wall_seen(h, [28.0, 15.0, 8.5], 3.0),
        [36.0 + 4.0 * std::f64::consts::FRAC_1_SQRT_2, 26.0 + 4.0 * std::f64::consts::FRAC_1_SQRT_2, 5.0],
    ] {
        h.press3d(at, egui::Modifiers::NONE);
        enter(h);
    }
    let yaw = h.app.viewing.cam.yaw;
    h.app.viewing.cam.yaw = 1.25 * std::f64::consts::PI;
    h.press3d([1.0, 1.0, 5.0], egui::Modifiers::NONE);
    enter(h);
    h.app.viewing.cam.yaw = yaw;
    h.key(egui::Key::Escape);
    h.key(egui::Key::Escape);
    at_rest(h);
}

#[test]
#[ignore = "draws the pictures of a manual check; run by hand"]
fn tutorial_shots_recognise_feature() {
    let mut shots = Shots::new("17-recognise-feature");
    let file = stl_of(&sample(), "test");
    let mut app = window();
    let mut h = Hand::new(&mut app);
    brought_in(&mut h, &file);
    h.look_at([20.0, 15.0, 5.0], 12.0);
    shots.take(&mut h, "mesh", None);

    // the mesh made a body as it is
    let at = h.button_of(&crate::i18n::tr("tb-recognise-hint"));
    shots.take(&mut h, "recognise-button", at);
    if let Some(at) = at {
        h.click_screen(at);
    }
    let on = point_at(&mut h, [20.0, 5.0, 10.0]);
    h.press3d([20.0, 5.0, 10.0], egui::Modifiers::NONE);
    h.press_word(&crate::i18n::tr("cmd-recognise-asis"), egui::pos2(700.0, 60.0));
    shots.take(&mut h, "recognise-as-is", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "polyhedron", None);

    // the through hole
    let at = h.button_of(&crate::i18n::tr("tb-recognise-feature-hint"));
    shots.take(&mut h, "recognise-feature-button", at);
    if let Some(at) = at {
        h.click_screen(at);
    }
    let wall = wall_seen(&h, [10.0, 15.0, 8.0], 5.0);
    let on = point_at(&mut h, wall);
    shots.take(&mut h, "hole-under-pointer", Some(on));
    h.press3d(wall, egui::Modifiers::NONE);
    shots.take(&mut h, "hole-taken", Some(on));
    type_in(&mut h, "f-diameter", 8.0);
    shots.take(&mut h, "hole-typed-8", None);
    enter(&mut h);
    shots.take(&mut h, "hole-made", None);

    // the blind hole, the rounding, the chamfer
    let wall = wall_seen(&h, [28.0, 15.0, 8.5], 3.0);
    let on = point_at(&mut h, wall);
    h.press3d(wall, egui::Modifiers::NONE);
    shots.take(&mut h, "blind-taken", Some(on));
    enter(&mut h);
    let round = [40.0 - 4.0 + 4.0 * std::f64::consts::FRAC_1_SQRT_2, 30.0 - 4.0 + 4.0 * std::f64::consts::FRAC_1_SQRT_2, 5.0];
    let on = point_at(&mut h, round);
    h.press3d(round, egui::Modifiers::NONE);
    shots.take(&mut h, "rounding-taken", Some(on));
    enter(&mut h);
    // the chamfer stands at the corner away from the view: the view turned to look at it from the front left
    h.app.viewing.cam.yaw = 1.25 * std::f64::consts::PI;
    let chamfer = [1.0, 1.0, 5.0];
    let on = point_at(&mut h, chamfer);
    h.press3d(chamfer, egui::Modifiers::NONE);
    shots.take(&mut h, "chamfer-taken", Some(on));
    enter(&mut h);

    // a flat face is refused
    let flat = [20.0, 25.0, 10.0];
    let on = point_at(&mut h, flat);
    h.press3d(flat, egui::Modifiers::NONE);
    shots.take(&mut h, "flat-refused", Some(on));
    h.key(egui::Key::Escape);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "tree", None);

    // the hole opened again from its row of the tree, made 12
    let row = crate::i18n::tr1("feat-remade-hole-through", "d", "8");
    let at = h.written_at(&row).map(|r| r.center());
    h.double_click_word(&row);
    shots.take(&mut h, "hole-reopened", at);
    type_in(&mut h, "f-diameter", 12.0);
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "hole-made-12", None);
}
