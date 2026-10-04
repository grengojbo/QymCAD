//! A SCALED IMPORT COMES BACK AT ITS SCALE when the document is opened from its embedded source.
//!
//! A file can name the wrong unit, so an imported solid is taken at a factor the person answers, and the import
//! node keeps it (`Project::set_import_scale`). On opening, the live body is parsed again from the source kept
//! inside the document - at the file's own numbers - and the window puts the factor back on it. The headless
//! reopening read the source and dropped the factor: a 10 mm cube taken at 25.4 came back at 1000 mm^3 instead of
//! 16 387 064 mm^3, so every check built on `open_like_the_app` measured a document the window never shows.
use qymcad_core::feature::PLACE_IDENTITY;
use qymcad_core::model::Project;
use qymcad_kernel::{import_step, write_step, Shape};

#[test]
fn a_scaled_import_comes_back_at_its_scale() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/scaled-import"));
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let path = dir.join("cube.step");
    let cube = Shape::extrude(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0], 10.0).expect("a cube");
    write_step(&[(&cube, PLACE_IDENTITY)], &path.to_string_lossy()).expect("the STEP is written");

    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("cube.step", std::fs::read(&path).expect("the file reads"));
    let mesh = import_step(&path.to_string_lossy(), 0.5).expect("the STEP imports").remove(0).mesh;
    let body = p.add_mesh(mesh);
    p.import_tree_as_parts(vec![qymcad_core::model::ImportNode { name: "cube".into(), body: Some(body), ..Default::default() }], source, "cube");
    // the file was drawn in inches: every side 10 in, that is 254 mm
    assert!(p.set_import_scale(body, 25.4), "the cube is not an import");

    let shapes = qymcad_testkit::restore_import_shapes(&p);
    let live = shapes.get(&body).unwrap_or_else(|| panic!("the cube came back with no live body ({} raised)", shapes.len()));
    let want = 254.0_f64.powi(3);
    assert!((live.volume() - want).abs() < want * 1e-6, "the cube came back at {} mm^3, not at its scale ({want} mm^3)", live.volume());
}
