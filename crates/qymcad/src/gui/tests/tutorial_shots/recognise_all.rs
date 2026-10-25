//! The pictures of the check of "Recognise all": the plate of 60 x 40 x 10 drilled 6 and 5 through and 4 blind, one
//! corner rounded 4 and one cut back 3, come in from a STEP file.
use super::*;

#[test]
#[ignore = "draws the pictures of a manual check; run by hand"]
fn tutorial_shots_recognise_all() {
    let mut shots = Shots::new("24-recognise-all");
    let file = step_of(&super::super::every_feature_is_made_exact_by_hand::plate(), "plate-all");
    let mut app = window();
    let mut h = Hand::new(&mut app);
    brought_in(&mut h, &file);
    h.look_at([30.0, 20.0, 5.0], 9.0);
    take_tool(&mut h, &mut shots, "tb-recognise-all-hint", Some("button"));
    let on = click(&mut h, [40.0, 32.0, 10.0]);
    shots.take(&mut h, "found", Some(on));

    // the rounding's group switched off by its name, a hole left out by a click on it
    let head = crate::i18n::tr("msg-all-round").split('{').next().unwrap_or_default().to_string();
    let group = h.words_drawn().iter().find(|(t, r)| t.starts_with(&head) && r.left() > 340.0).map(|(_, r)| r.center());
    if let Some(at) = group {
        h.click_screen(at);
    }
    shots.take(&mut h, "group-off", group);
    let wall = wall_seen(&h, [30.0, 10.0, 8.0], 2.5);
    let on = click(&mut h, wall);
    shots.take(&mut h, "hole-left-out", Some(on));
    enter(&mut h);
    h.key(egui::Key::Escape);
    shots.take(&mut h, "laid", None);
    let at = reopen_row(&mut h, &crate::i18n::tr("feat-name-remade-all"));
    shots.take(&mut h, "reopened", at);
}
