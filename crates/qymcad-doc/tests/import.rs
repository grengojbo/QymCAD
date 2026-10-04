//! A FILE BROUGHT IN: a mesh with no unit and a solid both land as parts at the factor asked, every factor taken from
//! the file's own numbers; the import is one undo step, and a scaled solid comes back at its scale from the written
//! document; an assembly lands as its tree; a size no part a person imports has is told.
use qymcad_core::feature::PLACE_IDENTITY;
use qymcad_core::model::{Id, Project};
use qymcad_doc::{import, DocEngine, DocError};
use qymcad_kernel::{import_step, write_step, Shape};
use std::collections::HashMap;

fn out_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-import")).join(name);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

/// A 10 mm cube written as STEP, and the same cube as STL; their paths.
fn cube_files(dir: &std::path::Path) -> (String, String) {
    let step = dir.join("cube.step").to_string_lossy().into_owned();
    let stl = dir.join("cube.stl").to_string_lossy().into_owned();
    let cube = Shape::extrude(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0], 10.0).expect("a cube");
    write_step(&[(&cube, PLACE_IDENTITY)], &step).expect("the STEP is written");
    let mesh = import_step(&step, 0.5).expect("the STEP reads").remove(0).mesh;
    qymcad_io::export_stl(&[mesh], &stl).expect("the STL is written");
    (step, stl)
}

/// The largest side of a body's mesh as it stands in the world.
fn side(p: &Project, body: Id) -> f64 {
    import::world_span(p, &[body]).iter().copied().fold(0.0, f64::max)
}

fn same_name(s: &str) -> String {
    s.to_string()
}

#[test]
fn a_mesh_with_no_unit_lands_as_a_part_at_its_factor() {
    let (_, stl) = cube_files(&out_dir("mesh"));
    let mut d = DocEngine::blank();
    let came = d.import(&stl, 25.4, &same_name).expect("the STL comes in");
    assert!(came.unitless, "an STL was taken for a file that names its unit");
    assert_eq!(came.bodies.len(), 1, "one cube came in as {} bodies", came.bodies.len());
    assert!((came.span[0] - 10.0).abs() < 1e-6, "the file's own size was told as {:?}, not 10 mm", came.span);
    let body = came.bodies[0];
    assert!((side(d.project(), body) - 254.0).abs() < 1e-6, "the cube landed at {} mm, not 254", side(d.project(), body));
    assert_eq!(d.project().import_scale(body), Some(25.4), "the node does not keep the factor its mesh stands at");
    let root = came.root.expect("the cube came in under a part");
    assert!(
        d.project().component_bodies(root).contains(&body) || d.project().descendants(root).iter().any(|c| d.project().component_bodies(*c).contains(&body)),
        "the cube is not in the part it came in under"
    );
    assert!(d.project().bodies.iter().any(|b| b.id == body && !b.faces.is_empty()), "the cube came in with no faces");
    assert_eq!(d.history().undo_names(), vec!["name-import"], "the import is not one step");
    assert!(d.project().sources.iter().any(|s| s.name == "cube.stl" && !s.data.is_empty()), "the original was not embedded");

    assert!(d.undo().is_some());
    assert!(d.project().mesh_index(body).is_none(), "undo left the cube in the document");
    assert!(d.redo().is_some());
    assert!((side(d.project(), body) - 254.0).abs() < 1e-6, "redo put the cube back at {} mm, not 254", side(d.project(), body));
}

#[test]
fn a_solid_lands_live_at_its_factor_and_comes_back_at_it_from_the_file() {
    let dir = out_dir("solid");
    let (step, _) = cube_files(&dir);
    let mut d = DocEngine::blank();
    let came = d.import(&step, 25.4, &same_name).expect("the STEP comes in");
    assert!(!came.unitless, "a STEP was taken for a file with no unit");
    let body = came.bodies[0];
    let want = 254.0_f64.powi(3);
    let v = d.shape(body).map(|s| s.volume()).expect("the solid came in with no live body");
    assert!((v - want).abs() < want * 1e-6, "the solid landed at {v} mm^3, not {want}");
    assert!(d.report().errors.is_empty(), "the import did not build: {:?}", d.report().errors);
    assert!((side(d.project(), body) - 254.0).abs() < 0.5, "the shown cube stands at {} mm, not 254", side(d.project(), body));

    let path = dir.join("scaled.qcad").to_string_lossy().into_owned();
    let _ = std::fs::remove_file(&path);
    d.save(&path, "2026-10-04T12:00:00Z").expect("the document is written");
    let again = DocEngine::open(&path).expect("the document opens");
    let v = again.shape(body).map(|s| s.volume()).expect("the solid came back from the file with no live body");
    assert!((v - want).abs() < want * 1e-6, "the solid came back from the file at {v} mm^3, not at its scale");
}

#[test]
fn every_factor_is_taken_from_the_files_own_numbers() {
    let (step, stl) = cube_files(&out_dir("again"));
    let mut p = Project::default();
    p.new_document();
    let mut shapes = HashMap::new();
    let pieces = import::read_mesh(&stl, import::MeshFormat::Stl).expect("the STL reads");
    let mesh = import::land_mesh(&mut p, &stl, import::MeshFormat::Stl, pieces, &same_name);
    let tree = qymcad_kernel::read_exact_tree(qymcad_kernel::ExactFormat::Step, &step, 0.5).expect("the STEP reads");
    let solid = import::land_exact(&mut p, &mut shapes, &step, tree, &same_name);
    for f in [25.4, 0.1, 1.0] {
        let _ = import::apply_scale(&mut p, &mut shapes, &mesh, f);
        assert!(import::apply_scale(&mut p, &mut shapes, &solid, f), "a solid taken at a factor was not said to need a rebuild");
        let m = side(&p, mesh.bodies[0]);
        assert!((m - 10.0 * f).abs() < 1e-9 * f.max(1.0), "the mesh taken at {f} stands at {m} mm, not {}", 10.0 * f);
        let v = shapes[&solid.bodies[0]].volume();
        let want = (10.0 * f).powi(3);
        assert!((v - want).abs() < want * 1e-6, "the solid taken at {f} stands at {v} mm^3, not {want}");
    }
}

#[test]
fn an_assembly_lands_as_its_tree() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/Table_CNC_WORK.stp");
    let read = qymcad_kernel::read_exact_tree(qymcad_kernel::ExactFormat::Step, path, 0.5).expect("the example reads");
    let solids = read.bodies.len();
    let mut d = DocEngine::blank();
    let came = d.import(path, 1.0, &same_name).expect("the example comes in");
    assert_eq!(came.bodies.len(), solids, "{} solids of the file came in as {} bodies", solids, came.bodies.len());
    assert!(d.report().errors.is_empty(), "the example did not build: {:?}", d.report().errors);
    assert!(came.bodies.iter().all(|b| d.shape(*b).is_some()), "a body of the example came in with no live shape");
    let root = came.root.expect("the example came in under a component");
    let under = d.project().descendants(root).len();
    // every node of the file's tree a component: the root, its subassemblies and a part for every solid
    fn count(n: &[qymcad_core::model::ImportNode]) -> usize {
        n.iter().map(|x| 1 + count(&x.children)).sum()
    }
    let tree = count(&qymcad_kernel::document_tree(&read.nodes, &came.bodies, "Table_CNC_WORK"));
    assert_eq!(under + 1, tree, "the file's tree of {tree} nodes came in as {} components", under + 1);
    assert!(!import::out_of_size(came.span), "the example was told to be of no sensible size: {:?}", came.span);
}

#[test]
fn a_size_no_part_has_is_told() {
    assert!(import::out_of_size([0.5, 0.2, 0.1]), "half a millimetre passed for a part");
    assert!(import::out_of_size([20_000.0, 10.0, 10.0]), "twenty metres passed for a part");
    assert!(!import::out_of_size([254.0, 254.0, 254.0]), "a 254 mm cube was told to be of no sensible size");
}

#[test]
fn a_file_that_cannot_come_in_leaves_no_step() {
    let dir = out_dir("refused");
    let txt = dir.join("notes.txt").to_string_lossy().into_owned();
    std::fs::write(&txt, "not a model").expect("the file is written");
    let mut d = DocEngine::blank();
    let bodies = d.project().bodies.len();
    let r = d.import(&txt, 1.0, &same_name);
    assert_eq!(r, Err(DocError::File("import-unknown#notes.txt".into())));
    let r = d.import(&dir.join("none.step").to_string_lossy(), 1.0, &same_name);
    assert!(matches!(r, Err(DocError::File(ref code)) if code.starts_with("cad-file-not-found#")), "a missing STEP gave {r:?}");
    assert_eq!(d.project().bodies.len(), bodies, "a file that did not come in left bodies behind");
    assert!(d.history().undo_names().is_empty(), "a file that did not come in left a step");
}
