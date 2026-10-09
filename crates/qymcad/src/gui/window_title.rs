//! WHAT THE WINDOW CALLS ITSELF.
//!
//! It said `qymcad` - the identifier the program is stored under, not a name anybody would write down.
//! A window in a task bar is often all a person sees of a program while three others are open, and the
//! first thing they look for there is which DOCUMENT this window holds.
//!
//! So the title carries both, and says when there is unsaved work: a mark before the document name, the
//! way an editor does it. A person who is about to close a window has one second to notice.
//!
//! Composed by a pure function on purpose - the shape of the title is worth checking, and checking it
//! must not need a window.

/// SAY `title` TO THE WINDOW when it differs from the one `shown`: which file is open, and whether it holds unsaved work.
///
/// Sent only when it has changed. A viewport command every frame is a message to the window manager sixty times a
/// second for a string that moves a few times an hour, and on some of them it makes the title flicker.
pub(crate) fn say(title: String, shown: &mut String, ctx: &egui::Context) {
    if title != *shown {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
        *shown = title;
    }
}

/// The title for a document at `path`, changed or not.
///
/// The name is the file's STEM: `Filter-v2`, not `Filter-v2.qcad`. The extension is the program's own
/// business and says nothing to whoever is looking for their part among four windows.
pub(crate) fn window_title(path: Option<&str>, unsaved: bool) -> String {
    let Some(path) = path else {
        // Never saved: there is no name to show, and inventing one ("Document 1") would be a name the
        // person cannot find on disk afterwards.
        return match unsaved {
            true => format!("{} — {} *", super::APP_NAME, crate::i18n::tr("title-unsaved-document")),
            false => super::APP_NAME.to_string(),
        };
    };
    // BOTH SEPARATORS, BY HAND. `Path::file_stem` knows only the separator of the system it is running
    // on: on Linux a Windows path comes back whole, backslashes and all. The packaged build runs on
    // Windows, and the tests run here - taking both apart means the tests actually cover what a person
    // will see, instead of a shape that only happens to work on the machine that checked it.
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = file.rsplit_once('.').map(|(head, _)| head).unwrap_or(file);
    let mark = if unsaved { " *" } else { "" };
    format!("{} — {stem}{mark}", super::APP_NAME)
}

#[cfg(test)]
mod tests {
    use super::window_title;

    #[test]
    fn it_names_the_program_and_the_document() {
        assert_eq!(window_title(Some("/home/user/parts/Filter-v2.qcad"), false), "QymCAD — Filter-v2");
        assert_eq!(window_title(Some("/home/user/parts/Filter-v2.qcad"), true), "QymCAD — Filter-v2 *");
        // A Windows path is the ordinary case for the packaged build.
        assert_eq!(window_title(Some("C:\\parts\\Bracket.qcad"), false), "QymCAD — Bracket");
    }

    #[test]
    fn a_document_never_saved_shows_only_the_program() {
        // Nothing is invented: without a file there is no name a person could look for on disk.
        assert_eq!(window_title(None, false), "QymCAD");
        assert!(window_title(None, true).starts_with("QymCAD — "), "{}", window_title(None, true));
        assert!(window_title(None, true).ends_with(" *"), "unsaved work has to be visible in the title");
    }

    /// THE WINDOW ACTUALLY WEARS IT, and only when it changed.
    ///
    /// Measured through real frames: the command reaches the window manager exactly once for one change.
    /// Sending it every frame is sixty messages a second for a string that moves a few times an hour, and
    /// on some window managers that shows up as a flickering title.
    #[test]
    fn the_title_is_sent_once_per_change() {
        let mut app = crate::gui::screen_keys::tests::populated();
        let ctx = egui::Context::default();
        crate::gui::install_fonts(&ctx);
        // what the frame does: the dirty flag, the title, and saying it when it changed
        let keep = |app: &mut crate::gui::App, c: &egui::Context| {
            let dirty = qymcad_ui_state::is_dirty(&mut app.rebuild_ctx());
            super::say(super::window_title(app.disk.project_path.as_deref(), dirty), &mut app.disk.title_shown, c);
        };
        let raw = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0))), ..Default::default() };
        let titles = |out: &egui::FullOutput| -> Vec<String> {
            out.viewport_output
                .values()
                .flat_map(|v| v.commands.iter())
                .filter_map(|c| match c {
                    egui::ViewportCommand::Title(t) => Some(t.clone()),
                    _ => None,
                })
                .collect()
        };

        // The fixture holds a document with work in it that has never been saved, so the mark belongs there.
        crate::gui::set_project_path(&mut app.disk.project_path, &mut app.set, "/tmp/Bracket.qcad".into());
        let first = ctx.run_ui(raw(), |c| keep(&mut app, c.ctx()));
        assert_eq!(titles(&first), vec!["QymCAD — Bracket *".to_string()], "the window was not told the new title");

        let again = ctx.run_ui(raw(), |c| keep(&mut app, c.ctx()));
        assert!(titles(&again).is_empty(), "the title was sent a second time although nothing changed");

        // AND THE MARK GOES AWAY WHEN THE WORK IS SAVED. A mark that only ever appears is not a signal.
        app.disk.edits.saved_key = qymcad_ui_state::edit_key(&app.draw_ctx());
        let saved = ctx.run_ui(raw(), |c| keep(&mut app, c.ctx()));
        assert_eq!(titles(&saved), vec!["QymCAD — Bracket".to_string()], "the title still claims unsaved work");
    }

    /// THE EXTENSION IS NOT PART OF THE NAME. A title bar is narrow and often truncated from the right;
    /// five characters of `.qcad` on every window buy nothing and cost the end of the actual name.
    #[test]
    fn the_extension_is_left_out() {
        assert!(!window_title(Some("/x/Filter-v2.qcad"), false).contains(".qcad"));
    }
}
