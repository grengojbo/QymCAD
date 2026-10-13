//! A HOLE, A ROUNDING OR A CHAMFER CLICKED ON A BODY MADE OF A MESH IS MADE AGAIN EXACT, through the document: the mesh
//! of a plate whose feature size is known comes in, is made a body - a polyhedron of flat faces, or recognised into
//! exact surfaces - the feature is found at the click, and one node takes its faces off and lays the feature down by its
//! own tool. The body then holds what the exact plate held, and the size edited in the tree drives it. Built with the
//! live kernel.
//!
//! Tool x mode x mesh: failures are gathered and told together.
use qymcad_core::model::{Id, ImportNode, Project};
use qymcad_kernel::Shape;
use qymcad_meshfit::feature::{feature_at, Pick};

fn plate() -> Shape {
    Shape::extrude(&[0.0, 0.0, 40.0, 0.0, 40.0, 30.0, 0.0, 30.0], 10.0).expect("a plate")
}

/// A cylinder of `r` round (20, 15), from z = `from` up `h`.
fn bore(r: f64, from: f64, h: f64) -> Shape {
    let at = [1.0, 0.0, 0.0, 20.0, 0.0, 1.0, 0.0, 15.0, 0.0, 0.0, 1.0, from];
    Shape::cylinder(r, h).expect("a cylinder").transformed(&at).expect("placed")
}

/// The edge of the plate at x = 40, y = 30, by its id.
fn corner_edge(s: &Shape) -> u32 {
    s.edges_info().into_iter().find(|e| e.poly.iter().all(|p| (p[0] as f64 - 40.0).abs() < 1e-6 && (p[1] as f64 - 30.0).abs() < 1e-6)).map(|e| e.id).expect("the corner edge")
}

/// One feature of the matrix: the exact plate, where its feature is clicked, the size it is edited to and the volume
/// the plate then holds.
struct Case {
    name: &'static str,
    exact: fn() -> Shape,
    clicked: fn(&qymcad_core::geom::MeshFace) -> bool,
    size: &'static str,
    edited: &'static str,
    edited_volume: f64,
}

const BLOCK: f64 = 40.0 * 30.0 * 10.0;

fn cases() -> Vec<Case> {
    let wall = |f: &qymcad_core::geom::MeshFace| f.normal[2].abs() < 0.1 && ((f.centroid.x - 20.0).powi(2) + (f.centroid.y - 15.0).powi(2)).sqrt() < 5.01;
    let corner = |f: &qymcad_core::geom::MeshFace| f.centroid.x > 36.0 && f.centroid.y > 26.0 && f.normal[2].abs() < 0.1 && f.normal[0] < 0.999 && f.normal[1] < 0.999;
    let pi = std::f64::consts::PI;
    vec![
        Case { name: "hole through", exact: || plate().boolean(&bore(5.0, -1.0, 12.0), 0).expect("drilled"), clicked: wall, size: "diameter", edited: "6", edited_volume: BLOCK - pi * 9.0 * 10.0 },
        Case { name: "blind hole", exact: || plate().boolean(&bore(3.0, 4.0, 7.0), 0).expect("drilled"), clicked: wall, size: "depth", edited: "3", edited_volume: BLOCK - pi * 9.0 * 3.0 },
        Case {
            name: "rounding",
            exact: || {
                let p = plate();
                p.fillet_edges(4.0, &[corner_edge(&p)]).expect("rounded")
            },
            clicked: corner,
            size: "radius",
            edited: "2",
            edited_volume: BLOCK - (4.0 - pi) * 10.0,
        },
        Case {
            name: "chamfer",
            exact: || {
                let p = plate();
                p.chamfer_edges(3.0, &[corner_edge(&p)], &[], &[], &[]).expect("cut")
            },
            clicked: |f| f.normal[0] > 0.1 && f.normal[1] > 0.1,
            size: "dist",
            edited: "2",
            edited_volume: BLOCK - 2.0 * 10.0,
        },
    ]
}

/// The mode a mesh is made a body in.
#[derive(Clone, Copy, Debug)]
enum Mode {
    Polyhedron,
    Recognised,
}

fn run(case: &Case, mode: Mode, deflection: f64) -> Result<(), String> {
    let exact = (case.exact)();
    let want = exact.volume();
    let mesh = exact.tessellate_merged(deflection).ok_or("no mesh")?.mesh;
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("plate.stl", vec![1]);
    let src = p.add_mesh(mesh);
    p.import_tree_as_parts(vec![ImportNode { name: "plate".into(), body: Some(src), mesh: true, ..Default::default() }], source, "plate").ok_or("the mesh did not come in")?;
    let part = p.timeline.iter().find(|n| n.kind.owns_body(src)).and_then(|n| n.parent);
    p.set_active_component(part);
    let body = match mode {
        Mode::Polyhedron => p.add_mesh_solid(src, 0.0),
        Mode::Recognised => p.add_mesh_recognised(src, 1.0, 30.0, 0.0),
    };
    let (_, shapes) = qymcad_testkit::regenerate(&mut p);
    let built = shapes.get(&body).and_then(|s| s.tessellate_merged(0.01)).ok_or("the body is not built")?;
    let face = built.faces.iter().find(|f| (case.clicked)(f)).ok_or("no face to click")?;
    let pick = Pick { face: face.id, at: [face.centroid.x, face.centroid.y, face.centroid.z] };
    let found = feature_at(&built.mesh, &built.faces, pick, 1.0).map_err(|u| format!("nothing found: {u:?}"))?;
    let faces = p.regen_faces.get(&body).cloned().unwrap_or_default();
    let remake = found.remake(&faces).ok_or_else(|| format!("{found:?} names faces the body does not have"))?;
    let before = p.timeline.len();
    let node = p.add_remade_feature(body, remake);
    if p.timeline.len() != before + 1 {
        return Err(format!("made again in {} nodes, not one", p.timeline.len() - before));
    }
    let made = volume_of(&mut p, node)?;
    if (made - want).abs() > 1e-3 * want {
        return Err(format!("made again it holds {made} mm^3, the exact plate {want}"));
    }
    p.set_feat_dim(node, case.size, case.edited.into());
    let edited = volume_of(&mut p, node)?;
    if (edited - case.edited_volume).abs() > 1e-3 * case.edited_volume {
        return Err(format!("{} edited to {} holds {edited} mm^3, not {}", case.size, case.edited, case.edited_volume));
    }
    Ok(())
}

fn volume_of(p: &mut Project, node: Id) -> Result<f64, String> {
    let (report, shapes) = qymcad_testkit::regenerate(p);
    if !report.errors.is_empty() {
        return Err(format!("the rebuild went red: {:?}", report.errors));
    }
    shapes.get(&node).map(|s| s.volume()).ok_or_else(|| "the node is not built".into())
}

#[test]
fn every_feature_found_on_a_mesh_is_made_exact_and_edited() {
    let mut wrong = Vec::new();
    for case in cases() {
        for mode in [Mode::Polyhedron, Mode::Recognised] {
            for deflection in [0.1, 0.01] {
                if let Err(why) = run(&case, mode, deflection) {
                    wrong.push(format!("{} / {mode:?} / mesh {deflection}: {why}", case.name));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{} of {} went wrong:\n{}", wrong.len(), cases().len() * 4, wrong.join("\n"));
}
