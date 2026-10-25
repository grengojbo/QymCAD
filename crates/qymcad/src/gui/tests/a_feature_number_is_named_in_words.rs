//! A NUMBER OF A FEATURE IS NAMED IN WORDS in the properties: a box picked in the tree shows its sizes under "Feature
//! numbers" by the words of the catalogue, not by the keys the document stores them under.
//!
//! Reported behaviour: the numbers of a boss redrawn by a sketch were headed `height`, those of the features made exact
//! `diameter`, `diameter.2`, `depth.2`, `dist.3`.
use crate::gui::hand::Hand;
use crate::gui::App;

#[test]
fn the_numbers_of_a_box_are_headed_by_their_words() {
    let mut app = App { project: qymcad_core::model::Project::default(), ..Default::default() };
    app.project.new_document();
    let _ = app.project.add_box(40.0, 30.0, 10.0);
    crate::gui::io_jobs::regenerate_now(&mut app.rebuild_ctx());
    let part = app.project.components.iter().find(|c| c.parent.is_some()).map(|c| crate::i18n::name(&c.name)).expect("the part of the box");
    let mut h = Hand::new(&mut app);
    // the part entered as a person enters it, by a double click on its row: its features stand in the tree then
    assert!(h.double_click_word(&part), "the row of the part {part:?} is not in the tree");
    h.frame(Vec::new());
    let name = crate::i18n::tr("feat-name-box");
    let row = h
        .words_drawn()
        .iter()
        .find(|(t, r)| t.starts_with(&name) && r.left() < 340.0)
        .map(|(_, r)| r.center())
        .unwrap_or_else(|| panic!("the row of the box in the tree: {:?}", h.words_drawn().iter().filter(|(_, r)| r.left() < 340.0).map(|(t, _)| t.clone()).collect::<Vec<_>>()));
    h.click_screen(row);
    h.frame(Vec::new());
    let props: Vec<String> = h.words_drawn().iter().filter(|(_, r)| r.left() > 1000.0).map(|(t, _)| t.clone()).collect();
    assert!(props.contains(&crate::i18n::tr("fp-params")), "the feature numbers are not shown for the box: {props:?}");
    for key in ["dx", "dy", "dz"] {
        assert!(!props.iter().any(|t| t == key), "the number {key} is headed by its key: {props:?}");
        let word = qymcad_i18n::dim_words::dim_word(key);
        assert!(props.contains(&word), "the number {key} is not headed {word:?}: {props:?}");
    }
}
