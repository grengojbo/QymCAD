//! The pictures of the check of "Sketch from face": a plate with a boss (A), the same with its top rounded (B), a plate
//! with a pocket (C), a plate with a round boss (D) and a block (the stock), each come in from a STEP file.
use super::*;

/// A plate of 60 x 40 x 5 with a boss of 30 x 20 x 8 from (10, 10), its top rounded `r` when one is given.
fn bossed(r: Option<f64>) -> Shape {
    let s = block([60.0, 40.0, 5.0]).combined(&placed(&block([30.0, 20.0, 8.0]), [10.0, 10.0, 5.0]), 1).expect("fused");
    let Some(r) = r else { return s };
    let top: Vec<u32> = s.edges_info().into_iter().filter(|e| e.poly.iter().all(|p| (p[2] as f64 - 13.0).abs() < 1e-6)).map(|e| e.id).collect();
    s.filleted(r, &top).expect("rounded")
}

/// A plate of 60 x 40 x 10 with a pocket of 30 x 20, 4 deep, from (10, 10).
fn pocketed() -> Shape {
    block([60.0, 40.0, 10.0]).combined(&placed(&block([30.0, 20.0, 5.0]), [10.0, 10.0, 6.0]), 0).expect("cut")
}

/// A plate of 60 x 40 x 5 with a round boss of 12 across and 8 high at (30, 20).
fn round_bossed() -> Shape {
    block([60.0, 40.0, 5.0]).combined(&placed(&specimen::cylinder(6.0, 8.0).expect("a boss"), [30.0, 20.0, 5.0]), 1).expect("fused")
}

/// The boss of A with one edge of its top rounded 2.
fn one_edge_rounded() -> Shape {
    let s = bossed(None);
    let edge = s.edges_info().into_iter().find(|e| e.poly.iter().all(|p| (p[2] as f64 - 13.0).abs() < 1e-6 && (p[1] as f64 - 10.0).abs() < 1e-6)).map(|e| e.id).expect("the front edge of the top");
    s.filleted(2.0, &[edge]).expect("rounded")
}

/// A fresh window with `shape` brought in from a STEP file named `name`, looked at whole.
fn with(shape: &Shape, name: &str, app: &mut App) -> Written {
    let file = step_of(shape, name);
    let mut h = Hand::new(app);
    brought_in(&mut h, &file);
    file
}

fn sketch_only(h: &mut Hand) {
    h.press_word(&crate::i18n::tr("cmd-sketch-only"), egui::pos2(300.0, 56.0));
}

fn replace(h: &mut Hand) {
    h.press_word(&crate::i18n::tr("cmd-sketch-replace"), egui::pos2(300.0, 56.0));
}

#[test]
#[ignore = "draws the pictures of a manual check; run by hand"]
fn tutorial_shots_sketch_from_face() {
    let mut shots = Shots::new("20-sketch-from-face");

    // A: sketch only, then replace
    let mut app = window();
    let _a = with(&bossed(None), "a", &mut app);
    let mut h = Hand::new(&mut app);
    h.look_at([30.0, 20.0, 6.0], 9.0);
    take_tool(&mut h, &mut shots, "tb-sketch-from-face-hint", Some("button"));
    sketch_only(&mut h);
    let on = click(&mut h, [25.0, 20.0, 13.0]);
    shots.take(&mut h, "a-sketch-only-taken", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "a-sketch-only-laid", None);
    // the sketch opens flat: a ring on its row of the tree would stand over the sheet
    let _ = reopen_row(&mut h, &format!("{} ", crate::i18n::tr("io-sketch")));
    shots.take(&mut h, "a-sketch-opened", None);
    h.chord(egui::Modifiers::COMMAND, egui::Key::Enter);
    h.undo();
    at_rest(&mut h);
    take_tool(&mut h, &mut shots, "tb-sketch-from-face-hint", None);
    replace(&mut h);
    let on = click(&mut h, [25.0, 20.0, 13.0]);
    shots.take(&mut h, "a-replace-taken", Some(on));
    type_in(&mut h, "f-redrawn-height", 10.0);
    shots.take(&mut h, "a-replace-typed-10", None);
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "a-replaced", None);
    let at = reopen_row(&mut h, &crate::i18n::tr("feat-name-redrawn-boss"));
    shots.take(&mut h, "a-reopened-sizes", at);
    type_in(&mut h, "f-redrawn-width", 40.0);
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "a-width-40", None);
    drop(h);

    // B: the rim rounded 2 is named and kept
    let mut app = window();
    let _b = with(&bossed(Some(2.0)), "b", &mut app);
    let mut h = Hand::new(&mut app);
    h.look_at([30.0, 20.0, 6.0], 9.0);
    take_tool(&mut h, &mut shots, "tb-sketch-from-face-hint", None);
    replace(&mut h);
    let on = click(&mut h, [25.0, 20.0, 13.0]);
    shots.take(&mut h, "b-rim-taken", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "b-replaced", None);
    drop(h);

    // C: the floor of the pocket made 6 deep
    let mut app = window();
    let _c = with(&pocketed(), "c", &mut app);
    let mut h = Hand::new(&mut app);
    h.look_at([30.0, 20.0, 5.0], 9.0);
    take_tool(&mut h, &mut shots, "tb-sketch-from-face-hint", None);
    replace(&mut h);
    let on = click(&mut h, [25.0, 20.0, 6.0]);
    shots.take(&mut h, "c-floor-taken", Some(on));
    type_in(&mut h, "f-redrawn-height", 6.0);
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "c-deeper", None);
    drop(h);

    // D: the round boss, its diameter beside its height when opened again; its round wall refused
    let mut app = window();
    let _d = with(&round_bossed(), "d", &mut app);
    let mut h = Hand::new(&mut app);
    h.look_at([30.0, 20.0, 6.0], 9.0);
    take_tool(&mut h, &mut shots, "tb-sketch-from-face-hint", None);
    replace(&mut h);
    let wall = wall_seen(&h, [30.0, 20.0, 9.0], -6.0);
    let on = click(&mut h, wall);
    shots.take(&mut h, "d-wall-refused", Some(on));
    let on = click(&mut h, [30.0, 20.0, 13.0]);
    shots.take(&mut h, "d-top-taken", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    let at = reopen_row(&mut h, &crate::i18n::tr("feat-name-redrawn-boss"));
    shots.take(&mut h, "d-reopened-diameter", at);
    type_in(&mut h, "f-redrawn-diameter", 16.0);
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "d-diameter-16", None);
    drop(h);

    // the block: a stock, 60 -> 80
    let mut app = window();
    let _cube = with(&block([60.0, 40.0, 5.0]), "plate", &mut app);
    let mut h = Hand::new(&mut app);
    h.look_at([30.0, 20.0, 2.0], 9.0);
    take_tool(&mut h, &mut shots, "tb-sketch-from-face-hint", None);
    replace(&mut h);
    let on = click(&mut h, [30.0, 20.0, 5.0]);
    shots.take(&mut h, "plate-stock-taken", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "plate-stock", None);
    drop(h);

    // one edge of the top rounded: refused
    let mut app = window();
    let _e = with(&one_edge_rounded(), "one-edge", &mut app);
    let mut h = Hand::new(&mut app);
    h.look_at([30.0, 20.0, 6.0], 9.0);
    take_tool(&mut h, &mut shots, "tb-sketch-from-face-hint", None);
    replace(&mut h);
    let on = click(&mut h, [25.0, 22.0, 13.0]);
    shots.take(&mut h, "one-edge-refused", Some(on));
}
