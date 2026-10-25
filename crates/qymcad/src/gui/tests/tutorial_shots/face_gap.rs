//! The pictures of the check of "Size on body": the block of the check before, its features made exact, a face pushed
//! beside the chamfer, a feature taken off, the gap between top and bottom set, and a hole moved off a side.
use super::*;

#[test]
#[ignore = "draws the pictures of a manual check; run by hand"]
fn tutorial_shots_face_gap() {
    let mut shots = Shots::new("18-face-gap");
    let file = stl_of(&super::recognise::sample(), "test");
    let mut app = window();
    let mut h = Hand::new(&mut app);
    super::recognise::recognised(&mut h, &file);
    shots.take(&mut h, "four-recognised", None);

    // the front face pushed 10: the chamfer beside it stays 2
    let at = h.button_of(&crate::i18n::tr("tb-push-face-hint"));
    shots.take(&mut h, "push-face-button", at);
    if let Some(at) = at {
        h.click_screen(at);
    }
    let front = [20.0, 0.0, 5.0];
    let on = point_at(&mut h, front);
    h.press3d(front, egui::Modifiers::NONE);
    type_in(&mut h, "f-offset2", 10.0);
    shots.take(&mut h, "front-pushed-preview", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "front-pushed", None);
    h.undo();
    at_rest(&mut h);

    // the chamfer taken off
    let at = h.button_of(&crate::i18n::tr("tb-recognise-feature-hint"));
    if let Some(at) = at {
        h.click_screen(at);
    }
    h.press_word(&crate::i18n::tr("cmd-remade-off"), egui::pos2(400.0, 56.0));
    let yaw = h.app.viewing.cam.yaw;
    let pitch = h.app.viewing.cam.pitch;
    h.app.viewing.cam.yaw = 1.25 * std::f64::consts::PI;
    let chamfer = [1.0, 1.0, 5.0];
    let on = point_at(&mut h, chamfer);
    h.press3d(chamfer, egui::Modifiers::NONE);
    shots.take(&mut h, "chamfer-off-taken", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "chamfer-off", None);
    h.app.viewing.cam.yaw = yaw;
    h.undo();
    at_rest(&mut h);

    // the gap between the top and the bottom
    let at = h.button_of(&crate::i18n::tr("tb-face-gap-hint"));
    shots.take(&mut h, "face-gap-button", at);
    if let Some(at) = at {
        h.click_screen(at);
    }
    let top = [20.0, 25.0, 10.0];
    let on = point_at(&mut h, top);
    h.press3d(top, egui::Modifiers::NONE);
    shots.take(&mut h, "gap-top-taken", Some(on));
    h.look_from_below();
    let bottom = [20.0, 25.0, 0.0];
    let on = point_at(&mut h, bottom);
    h.press3d(bottom, egui::Modifiers::NONE);
    shots.take(&mut h, "gap-bottom-taken", Some(on));
    h.look_at([20.0, 15.0, 5.0], 12.0);
    h.app.viewing.cam.yaw = yaw;
    h.app.viewing.cam.pitch = pitch;
    type_in(&mut h, "f-gap", 15.0);
    shots.take(&mut h, "gap-typed-15", None);
    h.press_word(&crate::i18n::tr("cmd-gap-both"), egui::pos2(500.0, 56.0));
    shots.take(&mut h, "gap-both", None);
    h.press_word(&crate::i18n::tr("cmd-gap-one"), egui::pos2(500.0, 56.0));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "gap-made-15", None);

    // the big hole moved 8 off the front face
    if let Some(at) = h.button_of(&crate::i18n::tr("tb-face-gap-hint")) {
        h.click_screen(at);
    }
    let wall = wall_seen(&h, [10.0, 15.0, 12.0], 4.0);
    let on = point_at(&mut h, wall);
    h.press3d(wall, egui::Modifiers::NONE);
    shots.take(&mut h, "hole-wall-taken", Some(on));
    let front = [25.0, 0.0, 7.0];
    let on = point_at(&mut h, front);
    h.press3d(front, egui::Modifiers::NONE);
    shots.take(&mut h, "hole-front-taken", Some(on));
    type_in(&mut h, "f-gap", 8.0);
    shots.take(&mut h, "hole-typed-8", None);
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "hole-moved", None);
}
