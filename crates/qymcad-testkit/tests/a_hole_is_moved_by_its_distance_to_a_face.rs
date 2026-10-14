//! A HOLE IS MOVED BY ITS DISTANCE TO A FLAT FACE: a plate 40 x 30 x 10 drilled 10 across at (20, 15) - exact, or come
//! in as a mesh and made a polyhedron - the hole found at a click on its wall, a side picked, and one node drills it
//! again with its axis that far from the side: the plate holds as much as before, the wall stands where it was asked
//! to, and the distance edited in the tree moves it again. A face the axis runs into is refused. Built with the live
//! kernel.
//!
//! Body x side x distance: failures are gathered and told together.
use qymcad_kernel::specimen::Specimen;
use qymcad_core::feature::FaceKey;
use qymcad_core::geom::MeshFace;
use qymcad_core::model::{HoleMove, Id, ImportNode, Project};
use qymcad_kernel::Shape;
use qymcad_meshfit::feature::{feature_at, Pick};

fn drilled() -> Shape {
    let plate = qymcad_kernel::specimen::extrude(&[0.0, 0.0, 40.0, 0.0, 40.0, 30.0, 0.0, 30.0], 10.0).expect("a plate");
    let at = [1.0, 0.0, 0.0, 20.0, 0.0, 1.0, 0.0, 15.0, 0.0, 0.0, 1.0, -1.0];
    plate.combined(&qymcad_kernel::specimen::cylinder(5.0, 12.0).expect("a drill").moved(&at).expect("placed"), 0).expect("drilled")
}

fn key(f: &MeshFace) -> FaceKey {
    FaceKey { index: 0, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id }
}

/// The mode the plate comes in.
#[derive(Clone, Copy, Debug)]
enum Plate {
    Exact,
    Polyhedron,
}

/// The plate in a part of its own; its body.
fn plate(p: &mut Project, how: Plate) -> Id {
    p.new_document();
    match how {
        Plate::Exact => {
            let source = p.add_source("plate.step", vec![1]);
            let mesh = qymcad_kernel::specimen::mesh_merged(&drilled(), 0.01).expect("a mesh").mesh;
            let src = p.add_mesh(mesh);
            p.import_tree_as_parts(vec![ImportNode { name: "plate".into(), body: Some(src), mesh: true, ..Default::default() }], source, "plate").expect("in");
            let part = p.timeline.iter().find(|n| n.kind.owns_body(src)).and_then(|n| n.parent);
            p.set_active_component(part);
            p.add_mesh_recognised(src, 1.0, 30.0, 0.0)
        }
        Plate::Polyhedron => {
            let source = p.add_source("plate.stl", vec![1]);
            let src = p.add_mesh(qymcad_kernel::specimen::mesh_merged(&drilled(), 0.1).expect("a mesh").mesh);
            p.import_tree_as_parts(vec![ImportNode { name: "plate".into(), body: Some(src), mesh: true, ..Default::default() }], source, "plate").expect("in");
            let part = p.timeline.iter().find(|n| n.kind.owns_body(src)).and_then(|n| n.parent);
            p.set_active_component(part);
            p.add_mesh_solid(src, 0.0)
        }
    }
}

/// What a node built: its volume and where the axis of its round wall stands, as the middle of the wall's corners.
struct Made {
    volume: f64,
    axis: [f64; 2],
}

fn made(p: &mut Project, node: Id) -> Result<Made, String> {
    let (report, shapes) = qymcad_testkit::regenerate(p);
    if !report.errors.is_empty() {
        return Err(format!("the rebuild went red: {:?}", report.errors));
    }
    let shape = shapes.get(&node).ok_or("the node is not built")?;
    let built = qymcad_kernel::specimen::mesh_merged(shape, 0.01).ok_or("no mesh")?;
    // the wall: the triangles standing square to z inside the plate's sides
    let mut sum = [0.0, 0.0];
    let mut n = 0.0;
    for f in built.faces.iter().filter(|f| f.normal[2].abs() < 0.1 && f.centroid.x > 0.5 && f.centroid.x < 39.5 && f.centroid.y > 0.5 && f.centroid.y < 29.5) {
        for &t in &f.triangles {
            for v in built.mesh.triangle(t as usize) {
                sum[0] += v.x;
                sum[1] += v.y;
                n += 1.0;
            }
        }
    }
    if n == 0.0 {
        return Err("no round wall".into());
    }
    Ok(Made { volume: shape.volume(), axis: [sum[0] / n, sum[1] / n] })
}

/// One run: the side picked (by its outward normal), the distance asked, and where the axis then stands.
struct Run {
    side: [f64; 3],
    size: f64,
    axis: [f64; 2],
    edited: f64,
    axis_edited: [f64; 2],
}

fn run(how: Plate, r: &Run) -> Result<(), String> {
    let mut p = Project::default();
    let body = plate(&mut p, how);
    let (_, shapes) = qymcad_testkit::regenerate(&mut p);
    let built = shapes.get(&body).and_then(|s| qymcad_kernel::specimen::mesh_merged(s, 0.01)).ok_or("the body is not built")?;
    let wall = built.faces.iter().find(|f| f.normal[2].abs() < 0.1 && ((f.centroid.x - 20.0).powi(2) + (f.centroid.y - 15.0).powi(2)).sqrt() < 5.01).ok_or("no wall")?;
    let found = feature_at(&built.mesh, &built.faces, Pick { face: wall.id, at: [wall.centroid.x, wall.centroid.y, wall.centroid.z] }, 1.0).map_err(|u| format!("nothing found: {u:?}"))?;
    let faces = p.regen_faces.get(&body).cloned().unwrap_or_default();
    let hole = found.remake(&faces).ok_or("the hole names faces the body does not have")?;
    // the side itself, not a strip of the wall facing the same way: the largest face so turned
    let side = faces.iter().filter(|f| (0..3).map(|i| f.normal[i] * r.side[i]).sum::<f64>() > 0.999).max_by(|a, b| a.area.total_cmp(&b.area)).map(key).ok_or("no such side")?;
    let want = made(&mut p, body)?.volume;
    let node = p.add_hole_moved(body, HoleMove { hole, face: side, size: r.size });
    let m = made(&mut p, node)?;
    let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 0.05 && (a[1] - b[1]).abs() < 0.05;
    if (m.volume - want).abs() > 1e-3 * want || !near(m.axis, r.axis) {
        return Err(format!("moved {} it holds {} mm^3 with the axis at {:?}, not {want} at {:?}", r.size, m.volume, m.axis, r.axis));
    }
    p.set_feat_dim(node, "gap", r.edited.to_string());
    let m = made(&mut p, node)?;
    if !near(m.axis, r.axis_edited) {
        return Err(format!("edited to {} the axis stands at {:?}, not {:?}; it holds {} mm^3", r.edited, m.axis, r.axis_edited, m.volume));
    }
    Ok(())
}

#[test]
fn a_hole_moves_to_stand_its_distance_from_a_side() {
    let runs = [
        Run { side: [0.0, -1.0, 0.0], size: 10.0, axis: [20.0, 10.0], edited: 22.0, axis_edited: [20.0, 22.0] },
        Run { side: [-1.0, 0.0, 0.0], size: 8.0, axis: [8.0, 15.0], edited: 30.0, axis_edited: [30.0, 15.0] },
        Run { side: [1.0, 0.0, 0.0], size: 12.0, axis: [28.0, 15.0], edited: 6.0, axis_edited: [34.0, 15.0] },
    ];
    let mut wrong = Vec::new();
    for how in [Plate::Polyhedron, Plate::Exact] {
        for r in &runs {
            if let Err(e) = run(how, r) {
                wrong.push(format!("{how:?} / side {:?} to {}: {e}", r.side, r.size));
            }
        }
    }
    assert!(wrong.is_empty(), "{} of {} went wrong:\n{}", wrong.len(), runs.len() * 2, wrong.join("\n"));
}

#[test]
fn a_hole_measured_to_the_face_it_opens_in_is_refused() {
    let mut p = Project::default();
    let body = plate(&mut p, Plate::Polyhedron);
    let (_, shapes) = qymcad_testkit::regenerate(&mut p);
    let built = shapes.get(&body).and_then(|s| qymcad_kernel::specimen::mesh_merged(s, 0.01)).expect("built");
    let wall = built.faces.iter().find(|f| f.normal[2].abs() < 0.1 && ((f.centroid.x - 20.0).powi(2) + (f.centroid.y - 15.0).powi(2)).sqrt() < 5.01).expect("wall");
    let found = feature_at(&built.mesh, &built.faces, Pick { face: wall.id, at: [wall.centroid.x, wall.centroid.y, wall.centroid.z] }, 1.0).expect("found");
    let faces = p.regen_faces.get(&body).cloned().unwrap_or_default();
    let hole = found.remake(&faces).expect("its faces");
    let top = faces.iter().find(|f| f.normal[2] > 0.999).map(key).expect("the top");
    let node = p.add_hole_moved(body, HoleMove { hole, face: top, size: 5.0 });
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    let said = report.errors.iter().find(|(id, _)| *id == node).map(|(_, e)| format!("{e:?}"));
    assert_eq!(said.as_deref(), Some("HoleAxisAcrossFace"), "{:?}", report.errors);
}

/// A HOLE MOVED TO NO DISTANCE says that the distance is the fault, not the faces picked: the distance kept as an
/// expression that comes to 0 leaves the node red with words about the number. Reported behaviour: the words asked to
/// pick two parallel faces, and the faces picked were the right ones.
#[test]
fn a_hole_moved_to_no_distance_says_the_distance_is_wrong() {
    let mut p = Project::default();
    let body = plate(&mut p, Plate::Polyhedron);
    let (_, shapes) = qymcad_testkit::regenerate(&mut p);
    let built = shapes.get(&body).and_then(|s| qymcad_kernel::specimen::mesh_merged(s, 0.01)).expect("built");
    let wall = built.faces.iter().find(|f| f.normal[2].abs() < 0.1 && ((f.centroid.x - 20.0).powi(2) + (f.centroid.y - 15.0).powi(2)).sqrt() < 5.01).expect("wall");
    let found = feature_at(&built.mesh, &built.faces, Pick { face: wall.id, at: [wall.centroid.x, wall.centroid.y, wall.centroid.z] }, 1.0).expect("found");
    let faces = p.regen_faces.get(&body).cloned().unwrap_or_default();
    let hole = found.remake(&faces).expect("its faces");
    let front = faces.iter().filter(|f| f.normal[1] < -0.999).max_by(|a, b| a.area.total_cmp(&b.area)).map(key).expect("the front");
    let node = p.add_hole_moved(body, HoleMove { hole, face: front, size: 10.0 });
    p.set_feat_dim(node, "gap", "10 - 10".into());
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    let said = report.errors.iter().find(|(id, _)| *id == node).map(|(_, e)| format!("{e:?}"));
    assert_eq!(said.as_deref(), Some("HoleGapNotAboveZero"), "{:?}", report.errors);
}
