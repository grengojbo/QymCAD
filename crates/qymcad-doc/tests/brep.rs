//! THE LIVE BODIES OF A DOCUMENT THAT HAS JUST COME IN: an import comes back at the factor it was taken at, a file saved
//! with its live bodies opens with them and rebuilds nothing, a file saved without them rebuilds only those, and a raw
//! mesh gets its faces found on the mesh.
use qymcad_core::feature::PLACE_IDENTITY;
use qymcad_core::model::{Id, Project};
use qymcad_doc::{brep, regen};
use qymcad_kernel::{import_step, write_step, Shape};
use std::collections::HashMap;

/// A folder of the check's own under the workspace's `target`.
fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-brep")).join(name);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

/// A 40x30x10 block on XY, rebuilt. Returns the project, the block's body and the live shapes.
fn block() -> (Project, Id, HashMap<Id, Shape>) {
    let mut p = Project::default();
    p.new_document();
    let si = p.new_sketch("base");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "base");
    p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, qymcad_core::feature::Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("the contour");
    let e = p.add_extrude_multi(sid, vec![cid], 10.0, qymcad_core::feature::Reach::Forward, 0.0, vec![]);
    let body = p.finish_base_body(e, 1);
    let (mut shapes, mut seen) = (HashMap::new(), HashMap::new());
    let done = regen::run(&mut p, &mut shapes, &mut seen, None);
    assert!(done.report.errors.is_empty(), "the block did not build: {:?}", done.report.errors);
    (p, body, shapes)
}

#[test]
fn an_import_comes_back_at_the_factor_it_was_taken_at() {
    let path = folder("scaled").join("cube.step");
    let cube = Shape::extrude(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0], 10.0).expect("a cube");
    write_step(&[(&cube, PLACE_IDENTITY)], &path.to_string_lossy()).expect("the STEP is written");

    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("cube.step", std::fs::read(&path).expect("the file reads"));
    let mesh = import_step(&path.to_string_lossy(), 0.5).expect("the STEP imports").remove(0).mesh;
    let body = p.add_mesh(mesh);
    p.import_tree_as_parts(vec![qymcad_core::model::ImportNode { name: "cube".into(), body: Some(body), ..Default::default() }], source, "cube");

    let as_read: HashMap<Id, Shape> = brep::import_shapes(&p).into_iter().collect();
    let v = as_read.get(&body).expect("the cube has a live body").volume();
    assert!((v - 1000.0).abs() < 1e-6, "the cube taken as it is came back at {v} mm^3, not 1000");

    // the file was drawn in inches: every side 10 in, that is 254 mm
    assert!(p.set_import_scale(body, 25.4), "the cube is not an import");
    let scaled: HashMap<Id, Shape> = brep::import_shapes(&p).into_iter().collect();
    let want = 254.0_f64.powi(3);
    let v = scaled.get(&body).expect("the cube has a live body").volume();
    assert!((v - want).abs() < want * 1e-6, "the cube came back at {v} mm^3, not at its scale ({want} mm^3)");
}

#[test]
fn a_file_saved_with_its_live_bodies_opens_with_them() {
    let (p, body, shapes) = block();
    let path = folder("with-bodies").join("block.qcad");
    let blobs: Vec<(Id, Vec<u8>)> = shapes.iter().filter_map(|(id, s)| s.to_brep_bytes().map(|b| (*id, b))).collect();
    qymcad_io::save_project_with_brep(&p, &path.to_string_lossy(), &blobs).expect("the file is written");

    let qymcad_io::LoadedProject { mut project, breps } = qymcad_io::load_project_with_brep(&path.to_string_lossy()).expect("the file reads");
    project.ensure_document();
    let mut live = HashMap::new();
    let adopted = brep::adopt_loaded(&mut project, &mut live, brep::shapes_from_blobs(breps));
    assert!(adopted.missing.is_empty(), "bodies the file carries were taken for missing: {:?}", adopted.missing);
    assert!(!adopted.imports, "the block was taken for an import");
    assert!(brep::missing_brep(&project, &live).is_empty(), "the block opened with no live body behind it");
    assert_eq!(project.regen_faces.get(&body).map(|f| f.len()), Some(6), "the block's faces are not where references look for them");
    let v = live.get(&body).expect("the block's live body").volume();
    assert!((v - 12000.0).abs() < 1e-6, "the live body read back is {v} mm^3, not 12000");
}

#[test]
fn a_file_saved_without_live_bodies_rebuilds_only_them() {
    let (p, body, _) = block();
    let path = folder("without-bodies").join("block.qcad");
    qymcad_io::save_project(&p, &path.to_string_lossy()).expect("the file is written");

    let mut project = qymcad_io::load_project(&path.to_string_lossy()).expect("the file reads");
    project.ensure_document();
    let mut live: HashMap<Id, Shape> = HashMap::new();
    let adopted = brep::adopt_loaded(&mut project, &mut live, Vec::new());
    assert!(adopted.missing.is_empty(), "the file carries the block's mesh, yet it was taken for missing");
    assert_eq!(brep::mark_missing_brep(&mut project, &live), 1, "the one node with no live body was not marked");
    let mut seen = HashMap::new();
    let done = regen::run(&mut project, &mut live, &mut seen, None);
    assert!(done.report.errors.is_empty(), "the block did not build: {:?}", done.report.errors);
    assert!(done.report.built.iter().any(|(b, _)| *b == body), "the block was not rebuilt");
    assert!(brep::missing_brep(&project, &live).is_empty(), "the block is still without a live body");
}

#[test]
fn a_raw_mesh_gets_its_faces_found_and_a_live_body_keeps_its_own() {
    let (mut p, body, shapes) = block();
    let before = p.bodies[p.mesh_index(body).expect("a mesh")].faces.clone();
    let raw = p.add_mesh(qymcad_kernel::box_mesh(5.0, 5.0, 5.0, 0.1));
    let given = brep::detect_missing_faces(&mut p, &shapes);
    assert_eq!(given.iter().map(|(b, f)| (*b, f.len())).collect::<Vec<_>>(), vec![(raw, 6)], "the raw box did not get its six faces alone");
    assert_eq!(p.bodies[p.mesh_index(body).expect("a mesh")].faces, before, "the faces of a body with a live B-rep were found again on its mesh");
}
