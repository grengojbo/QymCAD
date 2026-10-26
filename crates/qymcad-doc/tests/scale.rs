//! A FILE TAKEN AT ANOTHER SIZE: a mesh brought in and a solid brought in are scaled by the factor asked on top of the
//! one they stand at, as one step undo takes back; a body built in the document has no factor to change and is
//! refused.
use qymcad_core::model::Id;
use qymcad_doc::export::ExportTarget;
use qymcad_doc::import::{world_span, MeshFormat};
use qymcad_doc::{DocEngine, DocError};

fn same_name(s: &str) -> String {
    s.to_string()
}

/// A folder of the check's own.
fn out_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-scale")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

/// A document with a box of 40 x 30 x 10; its body.
fn a_box(d: &mut DocEngine) -> Id {
    d.edit("box", |p| {
        let body = p.add_box(40.0, 30.0, 10.0);
        Ok(p.finish_base_body(body, 1))
    })
    .expect("the box is laid")
}

/// The box written to `file`, in a folder of its name, and brought into a fresh document.
struct Brought {
    d: DocEngine,
    body: Id,
}

fn brought(file: &str) -> Brought {
    let path = out_dir(file).join(file).to_string_lossy().into_owned();
    let mut src = DocEngine::blank();
    let _ = a_box(&mut src);
    if let Some(format) = MeshFormat::of_path(&path) {
        src.export_mesh(&path, format, ExportTarget::Project, 0.01, &same_name).expect("the mesh is written");
    } else {
        src.export_exact(&path, qymcad_kernel::ExactFormat::Step, ExportTarget::Project, &same_name).expect("the STEP is written");
    }
    let mut d = DocEngine::blank();
    let came = d.import(&path, 1.0, &same_name).expect("the file comes in");
    Brought { body: came.bodies[0], d }
}

#[test]
fn a_mesh_brought_in_is_taken_at_the_factor_asked() {
    let Brought { mut d, body } = brought("box.stl");
    assert!((world_span(d.project(), &[body])[0] - 40.0).abs() < 1e-6, "setup: the box comes in at 40");
    let to = d.rescale(body, 2.54).expect("the mesh is scaled");
    assert!((to - 2.54).abs() < 1e-12, "the factor it stands at is {to}");
    assert!((world_span(d.project(), &[body])[0] - 101.6).abs() < 1e-6, "the box is not 101.6 long: {:?}", world_span(d.project(), &[body]));
    // on top of the factor it stands at
    let to = d.rescale(body, 0.5).expect("scaled again");
    assert!((to - 1.27).abs() < 1e-12 && (world_span(d.project(), &[body])[0] - 50.8).abs() < 1e-6, "{to}");
    d.undo();
    assert!((world_span(d.project(), &[body])[0] - 101.6).abs() < 1e-6, "undo did not take the last factor back");
}

#[test]
fn a_solid_brought_in_is_taken_at_the_factor_asked() {
    let Brought { mut d, body } = brought("box.step");
    let before = d.shape(body).map(|s| s.volume()).expect("a live solid");
    assert!((before - 12_000.0).abs() < 1e-3, "setup: the box holds {before} mm^3");
    let _ = d.rescale(body, 2.0).expect("the solid is scaled");
    let after = d.shape(body).map(|s| s.volume()).expect("a live solid");
    assert!((after - 96_000.0).abs() < 1e-2, "a solid twice as big holds {after} mm^3, not 8 times as much");
}

#[test]
fn a_body_built_here_is_refused() {
    let mut d = DocEngine::blank();
    let body = a_box(&mut d);
    let steps = d.history().undo_names().len();
    assert!(matches!(d.rescale(body, 2.0), Err(DocError::Refused(code)) if code == "scale-built-body"), "a built body took a factor");
    assert_eq!(d.history().undo_names().len(), steps, "a refusal left a step");
}
