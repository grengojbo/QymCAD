//! The pictures of "Sketch from section": a plate of 40 x 30 x 10 drilled 10 at (20, 15), come in as an STL file, cut
//! through its middle.
use super::*;

#[test]
#[ignore = "draws the pictures of a manual check; run by hand"]
fn tutorial_shots_sketch_from_section() {
    let mut shots = Shots::new("31-sketch-from-section");
    let plate = drilled(block([40.0, 30.0, 10.0]), Drill { at: [20.0, 15.0], r: 5.0, from: -1.0, h: 12.0 });
    let file = stl_of(&plate, "plate");
    let mut app = window();
    let mut h = Hand::new(&mut app);
    brought_in(&mut h, &file);
    h.look_at([20.0, 15.0, 5.0], 12.0);
    take_tool(&mut h, &mut shots, "tb-sketch-from-section-hint", Some("button"));
    let on = click(&mut h, [5.0, 5.0, 10.0]);
    shots.take(&mut h, "section-taken", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "sketch-laid", None);
    // the sketch opens flat: a ring on its row of the tree would stand over the sheet
    let _ = reopen_row(&mut h, &format!("{} ", crate::i18n::tr("io-sketch")));
    shots.take(&mut h, "sketch-opened", None);
}
