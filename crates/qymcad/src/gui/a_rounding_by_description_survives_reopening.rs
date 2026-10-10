//! A ROUNDING GIVEN BY A DESCRIPTION KEEPS IT WHEN REOPENED.
//!
//! Reported behaviour: a plate 60 x 40 x 4 with its four vertical edges rounded R5 - the edges given as "where the
//! front or the back meets the left or the right" - opened from its file and its rounding double-clicked in the tree:
//! the field said the rounding failed ("too big a radius, or the edges?"), and Enter with R2 rounded every edge of the
//! plate, the row reading "all of them". Reopening took the picked edges of the reference alone; a description has
//! none, an empty pick is "every edge", and Enter wrote that empty pick over the description.
//!
//! The path is the person's: the document written to a file and opened, the row double-clicked, Enter.
#[cfg(test)]
mod tests {
    use crate::gui::a_component_stepped_into_is_not_lit::tests::{calm, double_click_at};
    use crate::gui::import_door::tests::{frame, running};
    use crate::gui::App;
    use qymcad_core::model::{Id, Project};
    use qymcad_core::refs::{Axis, Cardinality, Query, Ref};

    /// "Where the front or the back meets the left or the right": the four vertical edges of a box.
    fn vertical_edges() -> Query {
        let side = |axis, max| Box::new(Query::Extreme { axis, max });
        let between = |a: Box<Query>, b: Box<Query>| Box::new(Query::Between(a, b));
        let front_left = between(side(Axis::Y, false), side(Axis::X, false));
        let front_right = between(side(Axis::Y, false), side(Axis::X, true));
        let back_left = between(side(Axis::Y, true), side(Axis::X, false));
        let back_right = between(side(Axis::Y, true), side(Axis::X, true));
        Query::Union(Box::new(Query::Union(Box::new(Query::Union(front_left, front_right)), back_left)), back_right)
    }

    /// The plate with its vertical edges rounded R5, written to a file and opened in the window; the rounding's key.
    fn the_plate_opened(app: &mut App, ctx: &egui::Context) -> Id {
        let mut p = Project::default();
        p.new_document();
        let plate = p.add_box(60.0, 40.0, 4.0);
        let round = p.add_fillet_ref(plate, 5.0, Ref { query: vertical_edges(), expect: Cardinality::Some, hint: Default::default() });
        let folder = crate::gui::check_folder::tests::CheckFolder::new("rounding-by-description");
        let path = folder.file("rounding-by-description.qcad").to_string_lossy().into_owned();
        qymcad_io::save_project(&p, &path).expect("the document was written");
        let project = qymcad_io::load_project(&path).expect("the document opened");
        app.finish_project_load(path, project, Vec::new());
        crate::gui::io_jobs::ensure_brep(&mut app.rebuild_ctx());
        app.drain_bg_for_test();
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        calm(app, ctx);
        // AS A PERSON OPENS IT: the file holds meshes and faces but no edges, and opening does not rebuild, so the model
        // has none of the plate's edges. Reported behaviour: right after opening, the reopened rounding said "none of
        // the 0 named edges is left"; after any rebuild it worked - a check that rebuilt in its setup passed blind.
        app.project.regen_edges.clear();
        round
    }

    /// The volume of the body the rounding made, and whether the rounding stands red.
    struct Rounded {
        volume: f64,
        red: bool,
    }

    fn rounded(app: &App, round: Id) -> Rounded {
        let volume = app.live.shapes.get(&round).map(|s| s.volume()).unwrap_or(f64::NAN);
        Rounded { volume, red: app.project.regen_errors.contains_key(&round) }
    }

    #[test]
    fn a_rounding_by_description_keeps_it_when_reopened() {
        let (mut app, ctx) = running();
        let round = the_plate_opened(&mut app, &ctx);
        let before = rounded(&app, round);
        // the plate less four corners of a 5 x 5 square less the quarter disc, 4 high
        let want = 9600.0 - 4.0 * 25.0 * (1.0 - std::f64::consts::FRAC_PI_4) * 4.0;
        assert!(!before.red && (before.volume - want).abs() < 1e-3, "setup: the opened plate measures {} not {want}", before.volume);

        // into the part first, by its row, as a person goes in to reach its features
        let part = app.project.body_owner(round).and_then(|c| app.project.components.iter().find(|x| x.id == c)).map(|c| crate::i18n::name(&c.name)).expect("the part of the plate");
        let texts = frame(&mut app, &ctx, Vec::new());
        let at = texts.iter().find(|(t, _)| *t == part).map(|(_, r)| r.center()).unwrap_or_else(|| panic!("no row {part:?}"));
        double_click_at(&mut app, &ctx, at);
        let texts = frame(&mut app, &ctx, Vec::new());
        let row =
            texts.iter().find(|(t, _)| t.contains("R5")).map(|(_, r)| r.center()).unwrap_or_else(|| panic!("no row of the rounding among {:?}", texts.iter().map(|(t, _)| t).collect::<Vec<_>>()));
        double_click_at(&mut app, &ctx, row);
        assert_eq!(app.tools.cmd.edit, Some(round), "the double click did not reopen the rounding");
        let shown = app.tools.gsel.edges.len();
        assert_eq!(shown, 4, "the reopened rounding shows {shown} edges taken, not the four it rounds");
        for _ in 0..4 {
            let _ = frame(&mut app, &ctx, Vec::new());
        }
        calm(&mut app, &ctx);
        let said = app.project.regen_errors.get(&round).cloned();
        assert!(said.is_none(), "the reopened rounding, untouched, stands red: {said:?}");

        let _ = frame(&mut app, &ctx, vec![egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]);
        calm(&mut app, &ctx);
        let kept = app.project.timeline.iter().find(|n| n.id == round).map(|n| n.kind.clone());
        let Some(qymcad_core::feature::FeatureKind::Fillet { edges, .. }) = kept else { panic!("the rounding is gone") };
        assert_eq!(edges.query, vertical_edges(), "Enter wrote over the description");
        let after = rounded(&app, round);
        assert!(!after.red && (after.volume - want).abs() < 1e-3, "after Enter the plate measures {} not {want}", after.volume);
    }
}
