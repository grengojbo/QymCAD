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
    // THE FILE IS READ AWAY FROM THE WINDOW: the body comes when it comes, as a person waits for it. Measured in a run of
    // the whole crate, every check beside it: the frames counted out by `at_rest` ran out before the body came in.
    crate::gui::session::waiting::until("the printer's file to come in", || {
        at_rest(&mut hand);
        !hand.app.project.bodies.is_empty()
    });
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

/// THE PLANE CLICKED, NOT ONLY XY, XZ OR YZ: with the body held by its top, a click on its flat front takes the front
/// for the plane the section is moved off - the offset field opens at the middle of the body along the front's normal,
/// 15 behind it - and Enter at 5 behind lays the sketch, the plate's 40 x 10 side, on a datum plane off that face, so
/// the sketch follows the face.
#[test]
fn a_flat_face_clicked_is_the_plane_the_section_is_moved_off() {
    let Printed { mut app, _folder } = a_printed_plate();
    let mut hand = Hand::new(&mut app);
    let sketches = hand.app.project.sketches.len();
    hand.tool(qymcad_part::SKETCH_FROM_SECTION).frame(Vec::new());
    hand.click([5.0, 5.0, 10.0]).frame(Vec::new());
    hand.click([30.0, 0.0, 5.0]).frame(Vec::new());
    let offset = hand.app.tools.cmd.params.iter().find(|f| f.key == "offset").map(|f| f.val);
    assert!(offset.is_some_and(|v| (v + 15.0).abs() < 1e-9), "the offset off the front is {offset:?}, not -15: {}", hand.app.status);
    hand.set("offset", -5.0);
    hand.key(egui::Key::Enter);
    at_rest(&mut hand);
    assert_eq!(hand.app.project.sketches.len(), sketches + 1, "Enter laid no sketch: {}", hand.app.status);
    let si = hand.app.project.sketches.len() - 1;
    let qymcad_core::feature::SketchPlane::Datum(plane) = hand.app.project.sketches[si].plane else { panic!("the sketch does not lie on a datum plane: {:?}", hand.app.project.sketches[si].plane) };
    let def = hand.app.project.planes.iter().find(|p| p.id == plane).map(|p| p.def.clone());
    assert!(matches!(def, Some(qymcad_core::model::PlaneDef::OffsetFace { dist, .. }) if (dist + 5.0).abs() < 1e-9), "the datum plane is laid as {def:?}, not 5 off the front");
    let frame = hand.app.project.sketch_frame(si).expect("the sketch has a frame");
    assert!((frame.origin[1] - 5.0).abs() < 1e-6, "the section is not taken 5 behind the front: {:?}", frame.origin);
    let s = &hand.app.project.sketches[si];
    let lines = s.entities.iter().filter(|e| matches!(e.kind, qymcad_core::model::EntityKind::Line { .. })).count();
    assert_eq!((s.entities.len(), lines), (4, 4), "the section is not the plate's side of 40 x 10");
    assert_eq!(hand.app.project.sketch_dof(si).0, 0, "the sketch of the section is left free: {}", hand.app.status);
    hand.key(egui::Key::Escape);
}

/// A DATUM PLANE CLICKED with the body held is the plane the section is moved off: one 2 above XY, the field opens at
/// the middle of the body along it, 3, and the sketch lies on a datum plane off it.
#[test]
fn a_datum_plane_clicked_is_the_plane_the_section_is_moved_off() {
    let Printed { mut app, _folder } = a_printed_plate();
    let mut hand = Hand::new(&mut app);
    // laid where a rebuild puts it: a datum plane comes from the timeline already placed
    let def = qymcad_core::model::PlaneDef::OffsetBase { base: qymcad_core::feature::BasePlane::XY, dist: 2.0 };
    let datum = hand.app.project.add_plane(qymcad_core::model::WorkPlane { origin: [0.0, 0.0, 2.0], normal: [0.0, 0.0, 1.0], def, ..Default::default() });
    hand.frame(Vec::new());
    let sketches = hand.app.project.sketches.len();
    hand.tool(qymcad_part::SKETCH_FROM_SECTION).frame(Vec::new());
    hand.click([5.0, 5.0, 10.0]).frame(Vec::new());
    let half = qymcad_ui_state::plane_pick_half_size(&hand.app.painting());
    hand.click([-0.6 * half, -0.6 * half, 2.0]).frame(Vec::new());
    let offset = hand.app.tools.cmd.params.iter().find(|f| f.key == "offset").map(|f| f.val);
    assert!(offset.is_some_and(|v| (v - 3.0).abs() < 1e-9), "the offset off the datum plane is {offset:?}, not 3: {}", hand.app.status);
    hand.key(egui::Key::Enter);
    at_rest(&mut hand);
    assert_eq!(hand.app.project.sketches.len(), sketches + 1, "Enter laid no sketch: {}", hand.app.status);
    let si = hand.app.project.sketches.len() - 1;
    let qymcad_core::feature::SketchPlane::Datum(plane) = hand.app.project.sketches[si].plane else { panic!("the sketch does not lie on a datum plane") };
    let def = hand.app.project.planes.iter().find(|p| p.id == plane).map(|p| p.def.clone());
    assert!(
        matches!(def, Some(qymcad_core::model::PlaneDef::OffsetPlane { plane: src, dist }) if src == datum && (dist - 3.0).abs() < 1e-9),
        "the datum plane is laid as {def:?}, not 3 off the one clicked"
    );
    hand.key(egui::Key::Escape);
}

/// THE VIEW IS CUT WHERE THE SECTION IS TAKEN, before Enter: with the plate held and cut 5 up, what stands above the
/// plane is not drawn and the cut is capped - on the software picture and in the scene handed to the card alike - so a
/// person sees where it cuts. Esc puts the tool away, and the scene is whole again.
#[test]
fn the_view_is_cut_where_the_section_is_taken() {
    let Printed { mut app, _folder } = a_printed_plate();
    let mut hand = Hand::new(&mut app);
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 600.0));
    let picture = |app: &App| {
        let basis = app.viewing.cam.basis();
        qymcad_render::rasterize_3d(&app.painting(), rect, &basis, 1.0, 1.0).expect("a raster")
    };
    let scene = |app: &App| crate::gui::render_scene::gpu_scene_flat(&app.painting()).0.len();
    let differ = |a: &egui::ColorImage, b: &egui::ColorImage| a.pixels.iter().zip(&b.pixels).filter(|(p, q)| p != q).count();
    let (whole, whole_scene) = (picture(hand.app), scene(hand.app));
    hand.tool(qymcad_part::SKETCH_FROM_SECTION).frame(Vec::new());
    hand.click([5.0, 5.0, 10.0]).frame(Vec::new());
    let cut = hand.app.painting().cut().plane;
    assert!(cut.is_some_and(|(o, n)| (o[2] - 5.0).abs() < 1e-9 && (n[2] - 1.0).abs() < 1e-9), "the view is cut at {cut:?}, not 5 up along Z");
    let (cut_picture, cut_scene) = (picture(hand.app), scene(hand.app));
    let changed = differ(&whole, &cut_picture);
    assert!(changed > 2000, "the picture of the plate cut 5 up differs from the whole one in {changed} pixels");
    assert_ne!(cut_scene, whole_scene, "the scene for the card is not cut: {cut_scene} vertices, as whole");
    hand.key(egui::Key::Escape).frame(Vec::new());
    assert!(hand.app.painting().cut().plane.is_none(), "the view stays cut after Esc");
    // the camera the tool borrowed is given back, so the scene is compared, not the picture
    assert_eq!(scene(hand.app), whole_scene, "the scene is not whole again after Esc");
}
