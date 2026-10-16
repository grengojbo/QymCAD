use super::*;

/// THE SECTION VIEW: look inside and come back out.
pub(super) fn the_section_looks_inside_and_comes_back(app: &mut App, problems: &mut Vec<String>) {
    //
    // A section is a way to look, not an edit: it must leave NOTHING in the document. And it must
    // turn on by the same button a person uses: the logic used to live inside the panel button
    // where a hand could not reach it - a check either poked at the fields directly (that is,
    // checked itself) or did not check at all.
    {
        let doc_before = app.project.state_key();
        let nodes_before = app.project.timeline.len();
        app.toggle_section();
        if !app.side.section.pick {
            problems.push("section: the button was pressed and the plane pick did not start".into());
        }
        // the plane is picked by clicking a face - the way a person does it
        let body = app.project.bodies.iter().find(|b| !b.sheet).map(|b| b.id);
        if let (Some(body), Some(mi)) = (body, body.and_then(|b| app.project.mesh_index(b))) {
            let _ = body;
            if let Some(bb) = app.project.bodies[mi].mesh.bounds() {
                let basis = app.viewing.cam.basis();
                let top = [(bb.min.x + bb.max.x) * 0.5, (bb.min.y + bb.max.y) * 0.5, bb.max.z];
                let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(900.0, 700.0));
                let at = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect, basis: &basis }.at(top).0;
                app.viewport_3d_click_at(at, rect, &basis);
            }
        }
        if app.side.section.plane.is_none() {
            problems.push(format!("section: the click on a face did not set the plane; status: {}", app.status));
        }
        // TURNED OFF - AND NOT A TRACE LEFT
        app.toggle_section();
        if app.side.section.plane.is_some() || app.side.section.pick {
            problems.push("section: it was turned off and stayed on".into());
        }
        if app.project.timeline.len() != nodes_before {
            problems.push(format!("the section left a trace in the timeline: there were {nodes_before}, now {}", app.project.timeline.len()));
        }
        if app.project.state_key() != doc_before {
            problems.push("the section changed the document - it is a way to LOOK, not an edit".into());
        }
        check_all(app, "the section view", problems);
    }
}
