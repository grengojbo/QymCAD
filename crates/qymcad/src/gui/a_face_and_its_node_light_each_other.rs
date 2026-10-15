//! A FACE AND THE NODE THAT MADE IT LIGHT EACH OTHER: the floor of a hole clicked with nothing in hand lights the row of
//! the hole in the tree; the row of the hole clicked lights the faces of the hole on the body.
//!
//! Reported behaviour: "a circle selected and nothing lit in the tree - it is not clear what to edit; a row selected and
//! nothing lit on the model, only a double click shows it".
#[cfg(test)]
mod tests {
    use crate::gui::hand::Hand;
    use crate::gui::{App, Sel};

    /// The window with the block drilled, and the node of the hole.
    struct Drilled {
        app: App,
        hole: qymcad_core::model::Id,
    }

    /// The block of 20 x 20 x 10 in a part of its own, the part entered, a hole of 10 across and 3 deep drilled in the
    /// middle of its top.
    fn drilled() -> Drilled {
        let mut app = App::default();
        super::super::joint_flow::tests::add_part_at(&mut app, 0.0);
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        let body = app.project.mesh_id(0).expect("the block");
        if let Some(owner) = app.project.body_owner(body) {
            app.enter_component(owner);
        }
        let mi = app.project.mesh_index(body).expect("the mesh");
        let top = app.project.bodies[mi].faces.iter().find(|f| f.normal[2] > 0.9 && f.centroid.z > 9.9).expect("the top");
        let key = qymcad_core::feature::FaceKey { index: 0, centroid: [10.0, 10.0, 10.0], normal: top.normal, id: top.id };
        let hole = app.project.add_hole_typed(body, key, qymcad_core::model::HoleTool { kind: 0, diameter: 10.0, depth: 3.0, dia2: 0.0, depth2: 0.0 });
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        Drilled { app, hole }
    }

    #[test]
    fn the_floor_of_a_hole_clicked_lights_the_row_of_the_hole() {
        let Drilled { mut app, hole } = drilled();
        let ti = app.project.timeline.iter().position(|n| n.id == hole).expect("the hole's node");
        // the words of the row, the icon before them drawn apart
        let label = super::super::panels_tree::feature_row_label(&app.project, ti);
        let row = label.split_once(' ').map_or(label.clone(), |(_, words)| words.to_string());
        let mut h = Hand::new(&mut app);
        h.look_at([10.0, 10.0, 7.0], 8.0).frame(Vec::new());
        h.click([7.0, 11.5, 7.0]).frame(Vec::new());
        let Sel::Face(mi, fi) = h.app.chosen.sel else { panic!("the click took no face: {}", h.app.status) };
        assert_eq!(qymcad_ui_state::made_by::maker_of(&h.app.project, Sel::Face(mi, fi)), Some(hole), "the floor clicked is not one the hole made");
        assert!(h.row_lit(&row), "the floor of the hole is selected and the row {row:?} of the hole is not lit");
    }

    #[test]
    fn the_row_of_a_hole_clicked_lights_its_faces() {
        let Drilled { mut app, hole } = drilled();
        let ti = app.project.timeline.iter().position(|n| n.id == hole).expect("the hole's node");
        // the words of the row, the icon before them drawn apart
        let label = super::super::panels_tree::feature_row_label(&app.project, ti);
        let row = label.split_once(' ').map_or(label.clone(), |(_, words)| words.to_string());
        let lit = qymcad_scheme::a(app.scheme.pal.selected(), 150);
        let mut h = Hand::new(&mut app);
        h.look_at([10.0, 10.0, 7.0], 8.0).frame(Vec::new());
        assert!(h.press_word(&row, egui::pos2(0.0, 0.0)), "the row {row:?} is not on screen");
        h.frame(Vec::new());
        assert!(h.app.chosen.sel == Sel::Feature(ti), "the press did not select the row");
        assert!(h.mesh_corners_in(lit) > 0, "the row of the hole is selected and no face of the hole is lit on the body");
    }

    #[test]
    fn the_card_of_the_top_names_the_node_that_made_the_top_not_the_hole() {
        let Drilled { mut app, hole } = drilled();
        let hole_row = super::super::panels_tree::feature_row_label(&app.project, app.project.timeline.iter().position(|n| n.id == hole).expect("the hole's node"));
        let mut h = Hand::new(&mut app);
        h.look_at([10.0, 10.0, 7.0], 8.0).frame(Vec::new());
        h.click([2.0, 2.0, 10.0]).frame(Vec::new());
        let Sel::Face(mi, fi) = h.app.chosen.sel else { panic!("the click took no face: {}", h.app.status) };
        let maker = qymcad_ui_state::made_by::maker_of(&h.app.project, Sel::Face(mi, fi)).expect("the top has a maker");
        assert_ne!(maker, hole, "the top is told as made by the hole");
        let maker_row = super::super::panels_tree::feature_row_label(&h.app.project, h.app.project.timeline.iter().position(|n| n.id == maker).expect("its row"));
        let card = super::super::screen_keys::tests::frame_text(h.app, |a, c| a.properties_panel(c));
        assert!(card.iter().any(|t| t.contains(&maker_row)), "the card of the top does not name {maker_row:?}; it says {card:?}");
        assert!(!card.iter().any(|t| t.contains(&hole_row)), "the card of the top names the hole {hole_row:?}: {card:?}");
        assert!(card.iter().any(|t| t.contains(&crate::i18n::tr("props-edit"))), "the card of the top has no Edit button: {card:?}");
    }
}
