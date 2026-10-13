//! A FACE PUSHED BESIDE A FEATURE MADE EXACT LEAVES THE FEATURE ITS SIZE. A plate comes in as a mesh, is made a
//! polyhedron, and its chamfer, rounding or hole is made exact by one node; then a flat face of the plate is pushed.
//! A chamfer or a rounding made exact beside the pushed face is taken off, the face pushed and the feature laid down again
//! the same size, in the one node of the push; a hole goes on with the face it opens in. Built with the live kernel.
//!
//! Reported behaviour: the side face beside a chamfer made exact (leg 2) pushed out by 10 stretched the chamfer: the
//! push stood below the chamfer and carried its face along, 28.3 -> 169.7 mm^2.
//!
//! Feature x mode the mesh is made a body in: failures are gathered and told together.
use qymcad_core::feature::FaceKey;
use qymcad_core::geom::MeshFace;
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

/// One case of the matrix: the exact plate, where its feature is clicked, the flat face pushed, and by how much the
/// plate grows when the feature keeps its size.
struct Case {
    name: &'static str,
    exact: fn() -> Shape,
    clicked: fn(&MeshFace) -> bool,
    pushed: fn(&MeshFace) -> bool,
    dist: f64,
    grows: f64,
    edited: Option<Edited>,
    modes: &'static [Mode],
}

/// The size of the feature made exact edited after the push, and how much more the pushed plate then holds.
struct Edited {
    key: &'static str,
    to: &'static str,
    grows: f64,
}

/// The mode a mesh is made a body in.
#[derive(Clone, Copy, Debug)]
enum Mode {
    Polyhedron,
    Recognised,
}

const BOTH: &[Mode] = &[Mode::Polyhedron, Mode::Recognised];

fn cases() -> Vec<Case> {
    let corner = |f: &MeshFace| f.centroid.x > 36.0 && f.centroid.y > 26.0 && f.normal[2].abs() < 0.1 && f.normal[0] < 0.999 && f.normal[1] < 0.999;
    let wall = |f: &MeshFace| f.normal[2].abs() < 0.1 && ((f.centroid.x - 20.0).powi(2) + (f.centroid.y - 15.0).powi(2)).sqrt() < 5.01;
    let pi = std::f64::consts::PI;
    vec![
        Case {
            name: "chamfer, the side beside it",
            exact: || {
                let p = plate();
                p.chamfer_edges(3.0, &[corner_edge(&p)], &[], &[], &[]).expect("cut")
            },
            clicked: |f| f.normal[0] > 0.1 && f.normal[1] > 0.1,
            pushed: |f| f.normal[0] > 0.999,
            dist: 10.0,
            grows: 30.0 * 10.0 * 10.0,
            edited: Some(Edited { key: "dist", to: "1", grows: (9.0 - 1.0) / 2.0 * 10.0 }),
            modes: BOTH,
        },
        Case {
            name: "rounding, the side beside it",
            exact: || {
                let p = plate();
                p.fillet_edges(4.0, &[corner_edge(&p)]).expect("rounded")
            },
            clicked: corner,
            pushed: |f| f.normal[1] > 0.999,
            dist: 10.0,
            grows: 40.0 * 10.0 * 10.0,
            edited: Some(Edited { key: "radius", to: "2", grows: (16.0 - 4.0) * (1.0 - pi / 4.0) * 10.0 }),
            modes: BOTH,
        },
        Case {
            name: "hole through, the top it opens in",
            exact: || plate().boolean(&bore(5.0, -1.0, 12.0), 0).expect("drilled"),
            clicked: wall,
            pushed: |f| f.normal[2] > 0.999,
            dist: 5.0,
            grows: (40.0 * 30.0 - pi * 25.0) * 5.0,
            edited: None,
            // recognised, the top holding the hole comes out a free-form surface, and a push refuses a free-form face
            // beside a curved one
            modes: &[Mode::Polyhedron],
        },
    ]
}

fn key(f: &MeshFace) -> FaceKey {
    FaceKey { index: 0, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id }
}

fn run(case: &Case, mode: Mode) -> Result<(), String> {
    let exact = (case.exact)();
    let want = exact.volume() + case.grows;
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("plate.stl", vec![1]);
    let src = p.add_mesh(exact.tessellate_merged(0.1).ok_or("no mesh")?.mesh);
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
    let found = feature_at(&built.mesh, &built.faces, Pick { face: face.id, at: [face.centroid.x, face.centroid.y, face.centroid.z] }, 1.0).map_err(|u| format!("nothing found: {u:?}"))?;
    let faces = p.regen_faces.get(&body).cloned().unwrap_or_default();
    let remade = p.add_remade_feature(body, found.remake(&faces).ok_or("the feature names faces the body does not have")?);
    volume_of(&mut p, remade)?;
    // the face is clicked on the body as it stands, the feature made exact
    let pushed = p.regen_faces.get(&remade).and_then(|fs| fs.iter().find(|f| (case.pushed)(f)).cloned()).ok_or("no face to push")?;
    let before = p.timeline.len();
    let push = p.add_push_face(remade, key(&pushed), case.dist);
    if p.timeline.len() != before + 1 {
        return Err(format!("pushed in {} nodes, not one", p.timeline.len() - before));
    }
    let got = volume_of(&mut p, push)?;
    if (got - want).abs() > 1e-3 * want {
        return Err(format!("pushed {} it holds {got} mm^3, not {want}", case.dist));
    }
    // the size of the feature made exact still drives it, the push standing below it
    if let Some(e) = &case.edited {
        p.set_feat_dim(remade, e.key, e.to.into());
        let got = volume_of(&mut p, push)?;
        if (got - want - e.grows).abs() > 1e-3 * want {
            return Err(format!("{} edited to {} it holds {got} mm^3, not {}", e.key, e.to, want + e.grows));
        }
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
fn a_face_pushed_beside_a_feature_made_exact_keeps_its_size() {
    let mut wrong = Vec::new();
    for case in cases() {
        for &mode in case.modes {
            if let Err(why) = run(&case, mode) {
                wrong.push(format!("{} / {mode:?}: {why}", case.name));
            }
        }
    }
    let runs: usize = cases().iter().map(|c| c.modes.len()).sum();
    assert!(wrong.is_empty(), "{} of {runs} went wrong:\n{}", wrong.len(), wrong.join("\n"));
}
