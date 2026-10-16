use super::*;

/// A PRINTED PART COMES BACK AS A MESH: ITS HOLE MADE EXACT ─────────────────────
///
/// A part come from a printer's file is a mesh: its hole is a ring of flat strips whose size nobody can change. A
/// plate of 40 x 30 x 10 drilled 10 across, 500 mm off the housing, is written out as such a file and brought in,
/// made a body as it is, and the wall of its hole clicked with "Recognise feature": one node makes the hole round
/// and exact, and the diameter typed into it again drives the plate.
pub(super) fn a_printed_part_comes_back_as_a_mesh(app: &mut App, problems: &mut Vec<String>) {
    use crate::gui::hand::Hand;
    let pi = std::f64::consts::PI;
    use qymcad_kernel::specimen::Specimen;
    let plate = qymcad_kernel::specimen::extrude(&[500.0, 0.0, 540.0, 0.0, 540.0, 30.0, 500.0, 30.0], 10.0).expect("a plate");
    let at = [1.0, 0.0, 0.0, 520.0, 0.0, 1.0, 0.0, 15.0, 0.0, 0.0, 1.0, -1.0];
    let drilled = plate.combined(&qymcad_kernel::specimen::cylinder(5.0, 12.0).expect("a drill").moved(&at).expect("placed"), 0).expect("drilled");
    let folder = crate::gui::check_folder::tests::CheckFolder::new("user-case-print");
    let file = folder.file("printed.3mf").to_string_lossy().into_owned();
    qymcad_io::export_3mf(&[qymcad_kernel::specimen::mesh_merged(&drilled, 0.1).expect("a mesh").mesh], &file).expect("the printer's file is written");
    // THE WINDOW REBUILDS IN THE BACKGROUND: frames run, as they do by themselves, until nothing is being built
    let at_rest = |h: &mut Hand| {
        for _ in 0..600 {
            h.frame(Vec::new());
            h.app.drain_bg_for_test();
            if h.app.regen.busy.is_none() && h.app.regen.bg.is_empty() && !h.app.regen.wanted {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    // the chain goes on where it stood: the view and the context are given back once the plate is done
    let view = (app.viewing.mode_3d, app.viewing.cam);
    let mut hand = Hand::new(app);
    let was = qymcad_ui_state::current_ctx_id(&hand.app.active_path, &hand.app.project);
    hand.import(&file);
    at_rest(&mut hand);
    let mesh = hand.app.project.bodies.last().map(|b| b.id).expect("the mesh came in");
    if let Some(part) = hand.app.project.body_owner(mesh) {
        hand.app.enter_component(part);
    }
    check_all(hand.app, "the printed plate brought in as a mesh", problems);
    hand.look_at([520.0, 15.0, 5.0], 8.0);
    hand.tool(35).frame(Vec::new());
    hand.click([505.0, 5.0, 10.0]).frame(Vec::new());
    let as_it_is = crate::i18n::tr("cmd-recognise-asis");
    hand.press_word(&as_it_is, egui::pos2(0.0, 0.0));
    hand.enter();
    at_rest(&mut hand);
    check_all(hand.app, "the printed plate made a polyhedron", problems);
    hand.tool(qymcad_part::RECOGNISE_FEATURE).frame(Vec::new());
    hand.click([520.0, 20.0, 7.0]).frame(Vec::new());
    if hand.app.params.recognise.feature.is_none() {
        problems.push(format!("a click on the wall of the printed hole found no feature: {}", hand.app.status));
    }
    hand.enter();
    at_rest(&mut hand);
    check_all(hand.app, "the printed hole made exact", problems);
    let remade = hand.app.project.timeline.iter().position(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::Remade { .. }));
    let volume = |app: &App| {
        app.project.timeline.iter().rev().find_map(|n| match n.kind {
            qymcad_core::feature::FeatureKind::Remade { body, .. } => app.live.shapes.get(&body).map(|s| s.volume()),
            _ => None,
        })
    };
    match (remade, volume(hand.app)) {
        (Some(_), Some(v)) if (v - (12000.0 - pi * 25.0 * 10.0)).abs() < 1.0 => {}
        (r, v) => problems.push(format!("the printed hole made exact: node {r:?}, the plate holds {v:?} mm^3, not {}", 12000.0 - pi * 25.0 * 10.0)),
    }
    // the hole made 6 across by its own row of the tree, as a person changes it
    if let Some(ti) = remade {
        hand.key(egui::Key::Escape).frame(Vec::new());
        let row = crate::gui::panels_tree::feature_row_label(&hand.app.project, ti);
        if !hand.double_click_word(&row) {
            problems.push(format!("the row {row:?} of the hole made exact is not in the tree"));
        }
        hand.set("diameter", 6.0);
        hand.enter();
        at_rest(&mut hand);
        check_all(hand.app, "the diameter of the printed hole changed in the tree", problems);
        match volume(hand.app) {
            Some(v) if (v - (12000.0 - pi * 9.0 * 10.0)).abs() < 1.0 => {}
            v => problems.push(format!("the printed hole made 6 across: the plate holds {v:?} mm^3, not {}", 12000.0 - pi * 9.0 * 10.0)),
        }
    }
    hand.key(egui::Key::Escape).frame(Vec::new());
    // the hole moved 10 from the front side with "Size on body", as a person does it: its wall clicked, then the
    // front side - 15 from the axis - 10 typed, Enter: the plate holds as much, the wall stands about y = 10
    hand.tool(qymcad_part::FACE_GAP).frame(Vec::new());
    hand.click([520.0, 18.0, 9.0]).frame(Vec::new());
    hand.click([530.0, 0.0, 5.0]).frame(Vec::new());
    let field = hand.app.tools.cmd.params.iter().find(|p| p.key == "gap").map(|p| p.val);
    if field.is_none_or(|d| (d - 15.0).abs() > 0.05) {
        problems.push(format!("\"Size on body\" on the printed hole and the front side stands at {field:?}, not 15: {}", hand.app.status));
    }
    let nodes = hand.app.project.timeline.len();
    hand.set("gap", 10.0);
    hand.enter();
    at_rest(&mut hand);
    check_all(hand.app, "the printed hole moved 10 from the front", problems);
    let moved = hand.app.project.timeline.last().filter(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::HoleMoved { .. })).map(|n| n.id);
    let wall_y = moved
        .and_then(|id| hand.app.project.regen_faces.get(&id))
        .and_then(|fs| fs.iter().find(|f| f.normal[2].abs() < 0.1 && f.centroid.x > 515.0 && f.centroid.x < 525.0 && f.centroid.y > 1.0 && f.centroid.y < 29.0))
        .map(|f| f.centroid.y);
    let held = moved.and_then(|id| hand.app.live.shapes.get(&id)).map(|s| s.volume());
    match (held, wall_y) {
        (Some(v), Some(y)) if (v - (12000.0 - pi * 9.0 * 10.0)).abs() < 1.0 && (y - 10.0).abs() < 0.5 && hand.app.project.timeline.len() == nodes + 1 => {}
        (v, y) => problems.push(format!("the printed hole moved 10 from the front: {v:?} mm^3, its wall about y = {y:?}, {} nodes more", hand.app.project.timeline.len() - nodes)),
    }
    hand.key(egui::Key::Escape).frame(Vec::new());
    // the hole taken off, as a person does it: the tool switched to "Take off" in its bar, the wall clicked, Enter -
    // the plate whole again, one node more
    hand.tool(qymcad_part::RECOGNISE_FEATURE).frame(Vec::new());
    let off = crate::i18n::tr("cmd-remade-off");
    if !hand.press_word(&off, egui::pos2(0.0, 0.0)) {
        problems.push(format!("the bar of \"Recognise feature\" offers no {off:?}"));
    }
    // the hole is 6 across about (520, 10) now: its far wall is seen just under the top
    hand.click([520.0, 13.0, 9.0]).frame(Vec::new());
    if hand.app.params.recognise.feature.is_none() {
        problems.push(format!("a click on the wall of the printed hole made 6 across found no feature: {}", hand.app.status));
    }
    let nodes = hand.app.project.timeline.len();
    hand.enter();
    at_rest(&mut hand);
    check_all(hand.app, "the printed hole taken off", problems);
    let whole = hand
        .app
        .project
        .timeline
        .last()
        .filter(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::RemoveFace { .. }))
        .and_then(|n| n.kind.body())
        .and_then(|b| hand.app.live.shapes.get(&b))
        .map(|s| s.volume());
    match whole {
        Some(v) if (v - 12000.0).abs() < 1.0 && hand.app.project.timeline.len() == nodes + 1 => {}
        v => problems.push(format!("the printed hole taken off: the plate holds {v:?} mm^3 in {} nodes more, not 12000 in one", hand.app.project.timeline.len() - nodes)),
    }
    hand.key(egui::Key::Escape).frame(Vec::new());
    // the plate made 12 thick with "Size on body", as a person does it: its top clicked, the view turned, its
    // bottom clicked, 12 typed, Enter - the top moves, one node more
    hand.tool(qymcad_part::FACE_GAP).frame(Vec::new());
    hand.click([505.0, 5.0, 10.0]).frame(Vec::new());
    let seen = (hand.app.viewing.cam.yaw, hand.app.viewing.cam.pitch);
    hand.look_from_below();
    hand.click([505.0, 5.0, 0.0]).frame(Vec::new());
    (hand.app.viewing.cam.yaw, hand.app.viewing.cam.pitch) = seen;
    if hand.app.params.recognise.gap.len() != 2 {
        problems.push(format!("\"Size on body\" did not take the top and the bottom of the printed plate: {}", hand.app.status));
    }
    let nodes = hand.app.project.timeline.len();
    hand.set("gap", 12.0);
    hand.enter();
    at_rest(&mut hand);
    check_all(hand.app, "the printed plate made 12 thick", problems);
    let thick = hand
        .app
        .project
        .timeline
        .last()
        .filter(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::FaceGap { .. }))
        .and_then(|n| n.kind.body())
        .and_then(|b| hand.app.live.shapes.get(&b))
        .map(|s| s.volume());
    match thick {
        Some(v) if (v - 14400.0).abs() < 1.0 && hand.app.project.timeline.len() == nodes + 1 => {}
        v => problems.push(format!("the printed plate made 12 thick holds {v:?} mm^3 in {} nodes more, not 14400 in one", hand.app.project.timeline.len() - nodes)),
    }
    hand.key(egui::Key::Escape).frame(Vec::new());
    // a sketch taken off the top of the plate with "Sketch from face": nothing stands on it to redraw, so Enter
    // lays the sketch alone - its outline, fully defined - and the body stays as it is
    hand.tool(qymcad_part::SKETCH_FROM_FACE).frame(Vec::new());
    hand.click([505.0, 5.0, 12.0]).frame(Vec::new());
    let nodes = hand.app.project.timeline.len();
    let sketches = hand.app.project.sketches.len();
    hand.enter();
    at_rest(&mut hand);
    check_all(hand.app, "a sketch taken off the top of the printed plate", problems);
    let si = hand.app.project.sketches.len().checked_sub(1).filter(|_| hand.app.project.sketches.len() == sketches + 1);
    match si.map(|si| hand.app.project.sketch_dof(si)) {
        Some((0, 0)) if hand.app.project.timeline.len() == nodes + 1 => {}
        dof => problems.push(format!("the sketch taken off the printed plate stands {dof:?} in {} nodes more, not fully defined in one: {}", hand.app.project.timeline.len() - nodes, hand.app.status)),
    }
    hand.key(egui::Key::Escape).frame(Vec::new());
    if let Some(si) = si {
        a_circle_drawn_on_it_is_fully_defined(&mut hand, si, problems);
    }
    hand.app.enter_component(was);
    (hand.app.viewing.mode_3d, hand.app.viewing.cam) = view;
}

/// A CIRCLE DRAWN ON THE SKETCH TAKEN OFF THE PLATE, AND "FULLY DEFINE": the sketch opened from its row in the tree, a
/// circle of 3 drawn in the middle of the outline, the tool taken, the circle clicked and Enter - its diameter and its
/// place laid from the origin, and the sketch has no freedom left.
fn a_circle_drawn_on_it_is_fully_defined(hand: &mut crate::gui::hand::Hand, si: usize, problems: &mut Vec<String>) {
    let name = hand.app.project.sketches[si].name.clone();
    if !hand.double_click_word(&name) {
        problems.push(format!("the sketch {name:?} taken off the printed plate has no row to open it by"));
        return;
    }
    hand.frame(Vec::new());
    let pts = &hand.app.project.sketches[si].points;
    let n = pts.len().max(1) as f64;
    let middle = pts.iter().fold((0.0, 0.0), |(x, y), p| (x + p.x / n, y + p.y / n));
    hand.sk_tool(3).click2d(middle.0, middle.1).click2d(middle.0 + 3.0, middle.1);
    hand.sk_tool(0);
    let free = hand.app.project.sketch_dof(si).0;
    hand.hover2d(middle.0, middle.1);
    if !hand.press_hint(&crate::i18n::tr("tb-fully-define-hint")) {
        problems.push("the button of Fully define is not on the panel of the sketch".into());
        return;
    }
    hand.click2d(middle.0 + 3.0, middle.1).frame(Vec::new());
    hand.key(egui::Key::Enter).frame(Vec::new());
    check_all(hand.app, "a circle on the sketch of the printed plate fully defined", problems);
    match hand.app.project.sketch_dof(si) {
        (0, 0) if free > 0 => {}
        dof => problems.push(format!("the circle on the sketch of the printed plate stands {dof:?} after Fully define (it was free by {free}): {}", hand.app.status)),
    }
    hand.key(egui::Key::Escape).frame(Vec::new());
    hand.app.finish_sketch_edit();
}
