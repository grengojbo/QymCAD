//! A FILE LAID ON THE BED: a part a file draws 75 mm up - a module of a lamp drawn where it stands in its assembly -
//! comes in with its lowest point on z = 0 when asked, every body of it moved as one by the component the file came in
//! under, its shape and size untouched; taken at another factor it is laid again.
use qymcad_core::model::{Id, Project};
use qymcad_doc::import;
use qymcad_kernel::specimen::Specimen;
use std::collections::HashMap;

fn out_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-on-bed")).join(name);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

/// A cube of 10 standing from z = 75, written as STEP and as STL; their paths.
struct Files {
    step: String,
    stl: String,
}

fn lifted_cube(dir: &std::path::Path) -> Files {
    let step = dir.join("lifted.step").to_string_lossy().into_owned();
    let stl = dir.join("lifted.stl").to_string_lossy().into_owned();
    let cube = qymcad_kernel::specimen::extrude(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0], 10.0).expect("a cube");
    let up = cube.moved(&[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 75.0]).expect("lifted");
    qymcad_kernel::write_step_of(&up, &step).expect("the STEP is written");
    let mesh = qymcad_kernel::import_step(&step, 0.5).expect("the STEP reads").remove(0).mesh;
    qymcad_io::export_stl(&[mesh], &stl).expect("the STL is written");
    Files { step, stl }
}

/// The lowest point of `bodies` as they stand in the world, each placed by its part.
fn lowest(p: &Project, bodies: &[Id]) -> f64 {
    let mut low = f64::MAX;
    for &id in bodies {
        let Some(b) = p.mesh_index(id).and_then(|i| p.bodies[i].mesh.bounds()) else { continue };
        let w = p.body_world_transform(id);
        for corner in 0..8 {
            let q = [if corner & 1 == 0 { b.min.x } else { b.max.x }, if corner & 2 == 0 { b.min.y } else { b.max.y }, if corner & 4 == 0 { b.min.z } else { b.max.z }];
            low = low.min(w[8] * q[0] + w[9] * q[1] + w[10] * q[2] + w[11]);
        }
    }
    low
}

fn same_name(s: &str) -> String {
    s.to_string()
}

#[test]
fn a_mesh_drawn_up_in_the_air_is_laid_on_the_bed() {
    let files = lifted_cube(&out_dir("mesh"));
    let mut p = Project::default();
    p.new_document();
    let pieces = import::read_mesh(&files.stl, import::MeshFormat::Stl).expect("the STL reads");
    let landed = import::land_mesh(&mut p, &files.stl, import::MeshFormat::Stl, pieces, &same_name);
    assert!((lowest(&p, &landed.bodies) - 75.0).abs() < 1e-6, "setup: the cube does not stand from 75 as the file draws it");
    let span = import::world_span(&p, &landed.bodies);
    let dropped = import::lay_on_bed(&mut p, landed.as_read());
    assert!((dropped - 75.0).abs() < 1e-6, "the cube was moved down {dropped}, not 75");
    assert!(lowest(&p, &landed.bodies).abs() < 1e-6, "the cube laid on the bed stands from {}", lowest(&p, &landed.bodies));
    assert_eq!(import::world_span(&p, &landed.bodies), span, "laying the cube on the bed changed its size");
    // taken at twice its size, from the file's numbers, it stands from 150 - and laid again, from 0
    let mut shapes = HashMap::new();
    let _ = import::apply_scale(&mut p, &mut shapes, landed.as_read(), 2.0);
    assert!((lowest(&p, &landed.bodies) - 150.0).abs() < 1e-6, "setup: the cube at 2 does not stand from 150");
    let _ = import::lay_on_bed(&mut p, landed.as_read());
    assert!(lowest(&p, &landed.bodies).abs() < 1e-6, "the cube at 2 laid on the bed stands from {}", lowest(&p, &landed.bodies));
}

#[test]
fn a_solid_drawn_up_in_the_air_is_laid_on_the_bed() {
    let files = lifted_cube(&out_dir("solid"));
    let mut p = Project::default();
    p.new_document();
    let mut shapes = HashMap::new();
    let tree = qymcad_kernel::read_exact_tree(qymcad_kernel::ExactFormat::Step, &files.step, 0.5).expect("the STEP reads");
    let landed = import::land_exact(&mut p, &mut shapes, &files.step, tree, &same_name);
    assert!((lowest(&p, &landed.bodies) - 75.0).abs() < 1e-6, "setup: the solid does not stand from 75 as the file draws it");
    let _ = import::lay_on_bed(&mut p, landed.as_read());
    assert!(lowest(&p, &landed.bodies).abs() < 1e-6, "the solid laid on the bed stands from {}", lowest(&p, &landed.bodies));
}
