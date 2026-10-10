//! DOCUMENTS: new, opened, saved, closed - as a person handles them.
use qymcad::{pos2, Session};
use qymcad_acceptance::probe;

/// A sample project of the repository.
const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/Filter-v2.qcad");

probe! {
    /// A PROJECT OPENED AND NOT TOUCHED CLOSES WITHOUT A QUESTION: there is nothing in it to save.
    fn a_project_opened_and_untouched_closes_without_a_question() {
        let mut s = Session::start();
        let file = s.word("menu-file");
        let file = s.find(&file, pos2(0.0, 0.0)).expect("the File menu is on screen");
        s.click(file.center());
        let open = s.word("file-open");
        let open = s.find(&open, file.center()).expect("Open project is in the File menu");
        s.click(open.center());
        let dont_save = s.word("nav-dont-save");
        if let Some(button) = s.find(&dont_save, pos2(0.0, 0.0)) {
            s.click(button.center());
        }
        s.answer_file(SAMPLE);
        let title = s.title();
        assert!(title.contains("Filter-v2") && !title.ends_with('*'), "a project just opened calls itself unsaved: {title:?}");
        if let Err(mut s) = s.quit() {
            panic!("a project opened and not touched asks about unsaved changes on closing; on screen: {:?}", s.words());
        }
    }
}

probe! {
    /// THE QUESTION ABOUT UNSAVED WORK HOLDS THE WINDOW: while it waits for an answer, nothing behind it takes a click.
    fn the_question_about_unsaved_work_holds_the_window() {
        let mut s = Session::start();
        qymcad_acceptance::build::into_the_first_part(&mut s);
        qymcad_acceptance::build::rectangle_on_xy(&mut s);
        let finish = s.word("wb-finish");
        s.press_word(&finish);
        let (file, new) = (s.word("menu-file"), s.word("file-new"));
        s.menu(&[&file, &new]);
        let question = s.word("nav-unsaved-title");
        assert!(s.shows(&question), "a new project over an unsaved sketch asks nothing; on screen: {:?}", s.words());
        let edit = s.word("sk-edit-btn");
        let behind = s.find(&edit, pos2(1280.0, 0.0)).unwrap_or_else(|| panic!("{edit:?} stands in the panel behind the question; on screen: {:?}", s.words()));
        s.click(behind.center());
        assert!(s.shows(&question), "a click behind the question put the question away");
        // the sketch opened is read from the document: "Finish" stands on screen in a part as well, for the part itself
        assert!(s.document().editing.is_none(), "a click on {edit:?} behind the question opened the sketch {:?} while the question waited", s.document().editing);
    }
}

/// A new project with a rectangle on XY, and Extrude taken on it: the sketch just finished is the contour it takes.
fn extrude_in_a_new_project(s: &mut Session) -> qymcad::Picture {
    let (file, new) = (s.word("menu-file"), s.word("file-new"));
    s.menu(&[&file, &new]);
    let dont = s.word("nav-dont-save");
    if let Some(r) = s.find(&dont, pos2(640.0, 400.0)) {
        s.click(r.center());
    }
    qymcad_acceptance::build::into_the_first_part(s);
    qymcad_acceptance::build::rectangle_on_xy(s);
    let finish = s.word("wb-finish");
    s.press_word(&finish);
    let extrude = s.word("tb-extrude-hint");
    s.press_hint(&extrude);
    s.snapshot()
}

probe! {
    /// NOTHING OF A CLOSED PROJECT IS DRAWN IN THE NEXT ONE. Reported behaviour: a project closed, a new one made, a
    /// sketch drawn and Extrude taken on it - and the green faces a check of a command had built in the closed project,
    /// a piece of its imported body, stood over the new sketch.
    fn nothing_of_a_closed_project_is_drawn_in_the_next() {
        let mut s = Session::start();
        qymcad_acceptance::build::block(&mut s);
        s.key(qymcad::Key::Escape);
        let hole = s.word("tb-hole-hint");
        s.press_hint(&hole);
        let at = s.face_at([20.0, 15.0, 10.0]);
        s.click(at);
        let _ = s.document();
        s.key(qymcad::Key::Escape).key(qymcad::Key::Escape);
        let after = extrude_in_a_new_project(&mut s);
        let mut clean = Session::start();
        let fresh = extrude_in_a_new_project(&mut clean);
        let canvas = s.canvas();
        if qymcad_acceptance::golden::changed_in(&fresh, &after, canvas, &[]) {
            let dir = std::env::temp_dir();
            let _ = std::fs::write(dir.join("closed-project-after.png"), after.png());
            let _ = std::fs::write(dir.join("closed-project-fresh.png"), fresh.png());
            panic!("Extrude in a new project is drawn otherwise after another project was worked in: see {}", dir.join("closed-project-after.png").display());
        }
    }
}

/// A file of this run for a check that saves, in the folder of temporary files.
fn saved_here(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("qymcad-{name}-{}-{:?}.qcad", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_file(&path);
    path.to_string_lossy().into_owned()
}

/// File -> Quit over unsaved work, answered "Save", as a person answers it.
fn quit_and_save(s: &mut Session) {
    let (file, quit) = (s.word("menu-file"), s.word("file-quit"));
    s.menu(&[&file, &quit]);
    let question = s.word("nav-unsaved-title");
    assert!(s.shows(&question), "Quit over unsaved work asks nothing; on screen: {:?}", s.words());
    let save = s.word("io-save");
    let button = s.find(&save, pos2(640.0, 400.0)).unwrap_or_else(|| panic!("the question has no {save:?}; on screen: {:?}", s.words()));
    s.click(button.center());
}

probe! {
    /// SAVE ON QUITTING SAVES AND QUITS. Reported behaviour: File -> Quit, "Save" - the file was written and the window
    /// stayed open, and Quit had to be chosen a second time.
    fn save_on_quitting_saves_and_quits() {
        let mut s = Session::start();
        qymcad_acceptance::build::into_the_first_part(&mut s);
        qymcad_acceptance::build::rectangle_on_xy(&mut s);
        let path = saved_here("quit-and-save");
        quit_and_save(&mut s);
        // a document never saved asks where to put it
        s.answer_file(&path);
        s.settle();
        assert!(std::path::Path::new(&path).exists(), "Save on quitting did not write {path}");
        assert!(s.closed(), "Save on quitting wrote the file and left the window open; on screen: {:?}", s.words());
    }
}

probe! {
    /// SAVE ON QUITTING A DOCUMENT THAT HAS A NAME writes it where it is, without asking where, and quits.
    fn save_on_quitting_a_named_document_saves_and_quits() {
        let mut s = Session::start();
        qymcad_acceptance::build::into_the_first_part(&mut s);
        qymcad_acceptance::build::rectangle_on_xy(&mut s);
        let path = saved_here("named-quit-and-save");
        let (file, save_as) = (s.word("menu-file"), s.word("file-save-as"));
        s.menu(&[&file, &save_as]);
        s.answer_file(&path);
        s.settle();
        let written = std::fs::metadata(&path).and_then(|m| m.modified()).expect("Save As wrote the file");
        // one more change after the name was given: a line in the sketch still open
        qymcad_acceptance::build::line(&mut s, (60.0, 0.0), (80.0, 0.0));
        assert!(s.title().ends_with('*'), "setup: the line left the document saved: {:?}", s.title());
        quit_and_save(&mut s);
        s.settle();
        assert!(s.chooser().is_none(), "a document with a name asked where to save it");
        let rewritten = std::fs::metadata(&path).and_then(|m| m.modified()).expect("the file is still there");
        assert!(rewritten > written, "Save on quitting did not write the document again");
        assert!(s.closed(), "Save on quitting wrote the file and left the window open; on screen: {:?}", s.words());
    }
}
