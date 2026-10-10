//! THE DOCUMENT PROPERTIES TRAVEL WITH THE FILE.
//!
//! A document used to be a nameless collection of geometry: no author, no title, no version. These
//! are properties of THE DOCUMENT rather than of the program — send it to a colleague and they see
//! whose it is and what it is — so they live in the `.qcad` and not in the config. The rule of
//! division: if it must travel with the file when the file is sent, its place is in the document.
#[cfg(test)]
mod tests {
    use crate::gui::check_folder::tests::CheckFolder;
    use super::super::App;
    use qymcad_core::model::{DocMeta, Project};

    fn filled() -> DocMeta {
        DocMeta {
            title: "Filter housing".into(),
            author: "Denis".into(),
            version: "rev. B".into(),
            comment: "print with a 0.4 nozzle".into(),
            created: "2026-08-02T10:00:00Z".into(),
            saved_by: String::new(),
        }
    }

    /// THE PROPERTIES SURVIVE WRITING AND READING THE DOCUMENT.
    #[test]
    fn the_properties_travel_with_the_document() {
        let mut p = Project::default();
        p.new_document();
        p.meta = filled();
        let text = ron::ser::to_string(&p).expect("the document serialises");
        let back: Project = ron::from_str(&text).expect("and reads back");
        assert_eq!(back.meta, filled(), "the document properties must survive writing and reading");
    }

    /// A DOCUMENT WITHOUT PROPERTIES STILL LOADS — files written BEFORE this work must open.
    ///
    /// The fixture is built by CUTTING the field out of a real record rather than by typing a minimal
    /// RON by hand: the document has plenty of fields without `serde(default)`, and a handwritten stub
    /// would check the wrong thing — it would fail on the first of those rather than on the absence of
    /// the properties.
    #[test]
    fn a_document_without_properties_still_loads() {
        let mut p = Project::default();
        p.new_document();
        p.meta = filled();
        let text = ron::ser::to_string(&p).expect("the document serialises");
        let at = text.find("meta:").expect("the properties field is there");
        let end = text[at..].find("units:").or_else(|| text[at..].find("next_id:")).expect("the next field") + at;
        let without = format!("{}{}", &text[..at], &text[end..]);

        let back: Project = ron::from_str(&without).expect("a document WITHOUT properties must read");
        assert!(back.meta.is_empty(), "a document without properties must have them empty rather than filled with rubbish");
        assert!(back.meta.created.is_empty(), "and the date too");
    }

    /// THE PROPERTIES ARE THE DOCUMENT, NOT A SETTING OF THE PROGRAM.
    ///
    /// A mistake in that direction costs dearly: had the author and the title gone into the config,
    /// they would have stuck to the machine, and one and the same file would be named differently for
    /// two people.
    #[test]
    fn the_properties_are_not_a_program_setting() {
        let text = ron::ser::to_string(&super::super::Settings::default()).expect("the settings serialise");
        for field in ["title", "author", "version", "comment"] {
            assert!(!text.contains(field), "the document property \"{field}\" leaked into the program settings: {text}");
        }
    }

    /// THE DATE IS STAMPED ONCE — "when it was created" is a fact, not a property of the last write.
    #[test]
    fn the_creation_date_is_stamped_once() {
        let mut app = App::default();
        assert!(app.project.meta.created.is_empty(), "setup: the new document has not been created yet");

        let folder = CheckFolder::new("doc-props-created");
        let dir = folder.path();
        let path = dir.join("props.qcad").to_string_lossy().into_owned();
        let _ = std::fs::remove_file(&path);

        app.save_for_test(path.clone());
        let first = app.project.meta.created.clone();
        assert!(!first.is_empty(), "the first save must stamp the date");

        // saved again, and the date is the same
        app.project.meta.created = first.clone();
        app.save_for_test(path.clone());
        assert_eq!(app.project.meta.created, first, "a repeated save must leave the creation date as it was");
        let _ = std::fs::remove_file(&path);
    }

    /// AN AUTOSAVE DOES NOT "CREATE" THE DOCUMENT — it is a snapshot, not the birth of a file.
    #[test]
    fn an_autosave_does_not_stamp_the_date() {
        let mut app = App::default();
        let folder = CheckFolder::new("doc-props-autosave");
        let dir = folder.path();
        let path = dir.join("auto.qcad").to_string_lossy().into_owned();
        let _ = std::fs::remove_file(&path);

        app.autosave_for_test(path.clone());
        assert!(app.project.meta.created.is_empty(), "an autosave does not create the document — the date must stay empty");
        let _ = std::fs::remove_file(&path);
    }
}
