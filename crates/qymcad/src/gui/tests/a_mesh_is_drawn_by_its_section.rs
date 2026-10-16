//! A BODY OF A MESH DRAWN BY ITS SECTION: a plate of 40 x 30 x 10 drilled 10 across, written as a printer's file and
//! brought in, clicked with "Sketch from section" - cut through its middle, the section counted in the bar - and Enter:
//! a sketch of a rectangle and a circle on a datum plane 5 up, defined to the end, its sizes the plate's.
use crate::gui::hand::Hand;
use crate::gui::App;
use qymcad_kernel::specimen::Specimen;

/// The window with a plate brought in, and the folder its file lies in - kept while the window is used.
struct Printed {
    app: App,
    _folder: crate::gui::check_folder::tests::CheckFolder,
}

/// The window with the printed plate brought in, its part entered, at rest.
fn a_printed_plate() -> Printed {
    let plate = qymcad_kernel::specimen::extrude(&[0.0, 0.0, 40.0, 0.0, 40.0, 30.0, 0.0, 30.0], 10.0).expect("a plate");
    let at = [1.0, 0.0, 0.0, 20.0, 0.0, 1.0, 0.0, 15.0, 0.0, 0.0, 1.0, -1.0];
    let drilled = plate.combined(&qymcad_kernel::specimen::cylinder(5.0, 12.0).expect("a drill").moved(&at).expect("placed"), 0).expect("drilled");
    let folder = crate::gui::check_folder::tests::CheckFolder::new("section-print");
    let file = folder.file("printed.3mf").to_string_lossy().into_owned();
    qymcad_io::export_3mf(&[qymcad_kernel::specimen::mesh_merged(&drilled, 0.05).expect("a mesh").mesh], &file).expect("the printer's file is written");
    let mut app = App::default();
    let mut hand = Hand::new(&mut app);
    hand.import(&file);
    at_rest(&mut hand);
    let mesh = hand.app.project.bodies.last().map(|b| b.id).expect("the mesh came in");
    if let Some(part) = hand.app.project.body_owner(mesh) {
        hand.app.enter_component(part);
    }
    hand.look_at([20.0, 15.0, 5.0], 8.0);
    drop(hand);
    Printed { app, _folder: folder }
}

/// Frames run, as they do by themselves, until nothing is being built.
fn at_rest(h: &mut Hand) {
    for _ in 0..600 {
        h.frame(Vec::new());
        h.app.drain_bg_for_test();
        if h.app.regen.busy.is_none() && h.app.regen.bg.is_empty() && !h.app.regen.wanted {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn the_plate_clicked_and_enter_lay_a_sketch_of_its_section() {
    let Printed { mut app, _folder } = a_printed_plate();
    let mut hand = Hand::new(&mut app);
    let sketches = hand.app.project.sketches.len();
    hand.tool(qymcad_part::SKETCH_FROM_SECTION).frame(Vec::new());
    hand.click([5.0, 5.0, 10.0]).frame(Vec::new());
    // before Enter: the section counted in the bar, nothing laid
    use qymcad_core::geom::{Fitted, Piece, Point2};
    let line = Piece::Line { from: Point2::new(0.0, 0.0), to: Point2::new(1.0, 0.0) };
    let hole = Piece::Circle { center: Point2::new(20.0, 15.0), radius: 5.0 };
    let counted = qymcad_part::told_pieces(&[Fitted { pieces: vec![line.clone(), line.clone(), line.clone(), line, hole], joins: Vec::new() }]);
    assert!(hand.texts().iter().any(|t| t.contains(&counted)), "the bar does not count the section as {counted:?}: {:?}", hand.texts());
    assert_eq!(hand.app.project.sketches.len(), sketches, "a sketch was laid before Enter");
    hand.key(egui::Key::Enter);
    at_rest(&mut hand);
    assert_eq!(hand.app.project.sketches.len(), sketches + 1, "Enter laid no sketch: {}", hand.app.status);
    let si = hand.app.project.sketches.len() - 1;
    assert!(matches!(hand.app.project.sketches[si].plane, qymcad_core::feature::SketchPlane::Datum(_)), "the sketch does not lie on a datum plane at the offset");
    assert_eq!(hand.app.project.sketch_dof(si).0, 0, "the sketch of the section is left free: {}", hand.app.status);
    let sizes: Vec<f64> = hand.app.project.sketches[si].constraints.iter().filter_map(|c| c.dim_value()).collect();
    for want in [10.0, 30.0, 40.0] {
        assert!(sizes.iter().any(|s| (s - want).abs() < 1e-9), "no size of {want} among {sizes:?}");
    }
    assert_eq!(hand.app.tools.armed.cmd_kind(), qymcad_part::SKETCH_FROM_SECTION, "the tool did not stay in hand");
    // ENTER AGAIN DRAWS NOTHING TWICE, and another offset is the next section with the fields held - the body and the
    // plane. Reported by the contract: the fields were read after the command was emptied, and the second Enter cut the
    // body at 0 with a tolerance of 0, along its bottom face: 49 lines.
    hand.key(egui::Key::Enter);
    at_rest(&mut hand);
    assert_eq!(hand.app.project.sketches.len(), sketches + 1, "the same section was drawn twice");
    hand.set("offset", 3.0);
    hand.key(egui::Key::Enter);
    at_rest(&mut hand);
    assert_eq!(hand.app.project.sketches.len(), sketches + 2, "another offset laid no sketch: {}", hand.app.status);
    let again = &hand.app.project.sketches[sketches + 1];
    let circles = again.entities.iter().filter(|e| matches!(e.kind, qymcad_core::model::EntityKind::Circle { .. })).count();
    assert_eq!((again.entities.len(), circles), (5, 1), "the second section is not the rectangle and the hole");
    hand.key(egui::Key::Escape);
}

/// "SKETCH FROM FACE" ON A FACE OF A MESH: the top of the printed plate clicked and Enter - its outline is chords, the
/// hole two dozen of them - is a sketch of the rectangle and one circle, defined to the end, not a polygon of the hole.
#[test]
fn the_top_of_the_printed_plate_is_drawn_with_its_hole_a_circle() {
    let Printed { mut app, _folder } = a_printed_plate();
    let mut hand = Hand::new(&mut app);
    let sketches = hand.app.project.sketches.len();
    hand.tool(qymcad_part::SKETCH_FROM_FACE).frame(Vec::new());
    hand.click([5.0, 5.0, 10.0]).frame(Vec::new());
    hand.key(egui::Key::Enter);
    at_rest(&mut hand);
    assert_eq!(hand.app.project.sketches.len(), sketches + 1, "Enter laid no sketch: {}", hand.app.status);
    let si = hand.app.project.sketches.len() - 1;
    let s = &hand.app.project.sketches[si];
    let circles = s.entities.iter().filter(|e| matches!(e.kind, qymcad_core::model::EntityKind::Circle { .. })).count();
    assert_eq!((s.entities.len(), circles), (5, 1), "the top is not drawn as its rectangle and its hole: {} curves, {circles} circles", s.entities.len());
    assert_eq!(hand.app.project.sketch_dof(si).0, 0, "the sketch of the top is left free: {}", hand.app.status);
}
