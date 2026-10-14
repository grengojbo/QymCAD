//! A SIZE BETWEEN TWO FLAT FACES OF A BODY DRIVES IT: two parallel faces picked, the distance between them typed, and one
//! node pushes the first, the second or both by half so they stand that far apart. The number stays in the tree and
//! drives the body when edited. A chamfer made exact beside a moved face keeps its size. Built with the live kernel.
//!
//! Body x which face moves x size: failures are gathered and told together.
use qymcad_kernel::specimen::Specimen;
use qymcad_core::feature::{FaceKey, GapMoves};
use qymcad_core::geom::MeshFace;
use qymcad_core::model::{Gap, Id, ImportNode, Project};
use qymcad_meshfit::feature::{feature_at, Pick};

fn key(f: &MeshFace) -> FaceKey {
    FaceKey { index: 0, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id }
}

fn face(p: &Project, body: Id, wanted: impl Fn(&MeshFace) -> bool) -> Result<FaceKey, String> {
    p.regen_faces.get(&body).and_then(|fs| fs.iter().find(|f| wanted(f))).map(key).ok_or_else(|| format!("no such face on body {body}"))
}

/// What a node built: its volume, and where its faces square to x stand.
struct Made {
    volume: f64,
    low_x: f64,
    high_x: f64,
}

fn made(p: &mut Project, node: Id) -> Result<Made, String> {
    let (report, shapes) = qymcad_testkit::regenerate(p);
    if !report.errors.is_empty() {
        return Err(format!("the rebuild went red: {:?}", report.errors));
    }
    let volume = shapes.get(&node).map(|s| s.volume()).ok_or("the node is not built")?;
    let fs = p.regen_faces.get(&node).cloned().unwrap_or_default();
    let at = |nx: f64| fs.iter().filter(|f| f.normal[0] * nx > 0.999).map(|f| f.centroid.x).next().ok_or("no face square to x");
    Ok(Made { volume, low_x: at(-1.0)?, high_x: at(1.0)? })
}

/// One run: a block 40 x 30 x 10 round the origin, its sides at x = -20 and x = 20 made `size` apart by moving `moves`.
struct Run {
    moves: GapMoves,
    size: f64,
    low_x: f64,
    high_x: f64,
}

fn block_run(run: &Run) -> Result<(), String> {
    let mut p = Project::default();
    p.new_document();
    let block = p.add_box(40.0, 30.0, 10.0);
    let _ = qymcad_testkit::regenerate(&mut p);
    let (one, other) = (face(&p, block, |f| f.normal[0] < -0.999)?, face(&p, block, |f| f.normal[0] > 0.999)?);
    let before = p.timeline.len();
    let node = p.add_face_gap(block, Gap { one, other, size: run.size, moves: run.moves });
    if p.timeline.len() != before + 1 {
        return Err(format!("laid down in {} nodes, not one", p.timeline.len() - before));
    }
    let m = made(&mut p, node)?;
    let want = run.size * 300.0;
    if (m.volume - want).abs() > 1e-6 * want || (m.low_x - run.low_x).abs() > 1e-6 || (m.high_x - run.high_x).abs() > 1e-6 {
        return Err(format!("it holds {} mm^3 from x = {} to {}, not {want} from {} to {}", m.volume, m.low_x, m.high_x, run.low_x, run.high_x));
    }
    // the size edited in the tree drives it: 45 apart, moved the same way
    p.set_feat_dim(node, "gap", "45".into());
    let m = made(&mut p, node)?;
    if (m.volume - 45.0 * 300.0).abs() > 1e-3 || ((m.high_x - m.low_x) - 45.0).abs() > 1e-6 {
        return Err(format!("edited to 45 it holds {} mm^3 from x = {} to {}", m.volume, m.low_x, m.high_x));
    }
    Ok(())
}

#[test]
fn a_size_between_two_faces_moves_the_face_asked_for() {
    let runs = [
        Run { moves: GapMoves::One, size: 50.0, low_x: -30.0, high_x: 20.0 },
        Run { moves: GapMoves::Other, size: 50.0, low_x: -20.0, high_x: 30.0 },
        Run { moves: GapMoves::Both, size: 50.0, low_x: -25.0, high_x: 25.0 },
        Run { moves: GapMoves::One, size: 25.0, low_x: -5.0, high_x: 20.0 },
        Run { moves: GapMoves::Other, size: 25.0, low_x: -20.0, high_x: 5.0 },
        Run { moves: GapMoves::Both, size: 25.0, low_x: -12.5, high_x: 12.5 },
    ];
    let wrong: Vec<String> = runs.iter().filter_map(|r| block_run(r).err().map(|e| format!("{:?} to {}: {e}", r.moves, r.size))).collect();
    assert!(wrong.is_empty(), "{} of {} went wrong:\n{}", wrong.len(), runs.len(), wrong.join("\n"));
}

#[test]
fn a_size_between_faces_not_parallel_is_refused_in_words() {
    let mut p = Project::default();
    p.new_document();
    let block = p.add_box(40.0, 30.0, 10.0);
    let _ = qymcad_testkit::regenerate(&mut p);
    let (one, other) = (face(&p, block, |f| f.normal[0] > 0.999).expect("a side"), face(&p, block, |f| f.normal[1] > 0.999).expect("the side beside"));
    let node = p.add_face_gap(block, Gap { one, other, size: 50.0, moves: GapMoves::One });
    let (report, _) = qymcad_testkit::regenerate(&mut p);
    let said = report.errors.iter().find(|(id, _)| *id == node).map(|(_, e)| format!("{e:?}"));
    assert_eq!(said.as_deref(), Some("GapNotParallel"), "two sides square to each other: {:?}", report.errors);
}

/// The plate of a mesh made a polyhedron, its chamfer of 3 at x = 40, y = 30 made exact; its sides at x = 0 and x = 40
/// made 50 apart by moving the side beside the chamfer. The chamfer keeps its leg: the plate grows by 30 x 10 x 10.
#[test]
fn a_size_beside_a_chamfer_made_exact_keeps_the_chamfer() {
    let plate = qymcad_kernel::specimen::extrude(&[0.0, 0.0, 40.0, 0.0, 40.0, 30.0, 0.0, 30.0], 10.0).expect("a plate");
    let edge = plate.edges_info().into_iter().find(|e| e.poly.iter().all(|q| (q[0] as f64 - 40.0).abs() < 1e-6 && (q[1] as f64 - 30.0).abs() < 1e-6)).map(|e| e.id).expect("the corner edge");
    let exact = plate.chamfered(3.0, &[edge]).expect("cut");
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("plate.stl", vec![1]);
    let src = p.add_mesh(qymcad_kernel::specimen::mesh_merged(&exact, 0.1).expect("a mesh").mesh);
    p.import_tree_as_parts(vec![ImportNode { name: "plate".into(), body: Some(src), mesh: true, ..Default::default() }], source, "plate").expect("the mesh came in");
    let part = p.timeline.iter().find(|n| n.kind.owns_body(src)).and_then(|n| n.parent);
    p.set_active_component(part);
    let body = p.add_mesh_solid(src, 0.0);
    let (_, shapes) = qymcad_testkit::regenerate(&mut p);
    let built = shapes.get(&body).and_then(|s| qymcad_kernel::specimen::mesh_merged(s, 0.01)).expect("the body is built");
    let f = built.faces.iter().find(|f| f.normal[0] > 0.1 && f.normal[1] > 0.1).expect("the face of the chamfer");
    let found = feature_at(&built.mesh, &built.faces, Pick { face: f.id, at: [f.centroid.x, f.centroid.y, f.centroid.z] }, 1.0).expect("the chamfer found");
    let faces = p.regen_faces.get(&body).cloned().unwrap_or_default();
    let remade = p.add_remade_feature(body, found.remake(&faces).expect("its faces"));
    made(&mut p, remade).expect("the chamfer made exact");
    let (one, other) = (face(&p, remade, |f| f.normal[0] < -0.999).expect("x = 0"), face(&p, remade, |f| f.normal[0] > 0.999).expect("x = 40"));
    let node = p.add_face_gap(remade, Gap { one, other, size: 50.0, moves: GapMoves::Other });
    let m = made(&mut p, node).expect("the size laid down");
    let want = exact.volume() + 3000.0;
    assert!((m.volume - want).abs() < 1e-3 * want && (m.high_x - 50.0).abs() < 1e-6, "it holds {} mm^3 to x = {}, not {want} to 50", m.volume, m.high_x);
}
