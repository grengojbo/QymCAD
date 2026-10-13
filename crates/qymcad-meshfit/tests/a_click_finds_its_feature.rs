//! A CLICK ON A BODY FINDS THE HOLE, THE ROUNDING OR THE CHAMFER UNDER IT, and its size. The bodies come from our
//! kernel, so the size is known: a plate 40 x 30 x 10 drilled 10 across, the same plate cut by a prism of N sides - the
//! hole a mesh draws with N flat strips - a blind hole, an edge rounded 4 and one cut back 3. What is clicked is one face
//! of the feature, at its centre, as the window gives it.

use qymcad_core::geom::{Built, MeshFace};
use qymcad_kernel::Shape;
use qymcad_meshfit::feature::{feature_at, rounded, rounded_bore, Found, Pick, Reach, Unfound};

fn plate() -> Shape {
    Shape::extrude(&[0.0, 0.0, 40.0, 0.0, 40.0, 30.0, 0.0, 30.0], 10.0).expect("a plate")
}

/// A prism of `sides` round (20, 15), its corners `r` from the axis, from z = `from` to z = `from + h`.
fn prism(sides: usize, r: f64, from: f64, h: f64) -> Shape {
    let xy: Vec<f64> = (0..sides)
        .flat_map(|k| {
            let a = std::f64::consts::TAU * k as f64 / sides as f64;
            [20.0 + r * a.cos(), 15.0 + r * a.sin()]
        })
        .collect();
    let up = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, from];
    Shape::extrude(&xy, h).expect("a prism").transformed(&up).expect("lifted")
}

/// A cylinder of `r` round (20, 15), from z = `from` up `h`.
fn bore(r: f64, from: f64, h: f64) -> Shape {
    let at = [1.0, 0.0, 0.0, 20.0, 0.0, 1.0, 0.0, 15.0, 0.0, 0.0, 1.0, from];
    Shape::cylinder(r, h).expect("a cylinder").transformed(&at).expect("placed")
}

fn built(s: &Shape) -> Built {
    s.tessellate_merged(0.01).expect("a body")
}

/// The face of `b` that `wanted` picks, clicked at its centre.
fn click(b: &Built, wanted: impl Fn(&MeshFace) -> bool) -> Pick {
    let f = b.faces.iter().find(|f| wanted(f)).expect("the face to click");
    Pick { face: f.id, at: [f.centroid.x, f.centroid.y, f.centroid.z] }
}

/// A face of the wall round (20, 15): square to z, its centre within `r` of the axis.
fn wall_of(r: f64) -> impl Fn(&MeshFace) -> bool {
    move |f| f.normal[2].abs() < 0.1 && ((f.centroid.x - 20.0).powi(2) + (f.centroid.y - 15.0).powi(2)).sqrt() < r + 0.01
}

fn id_of(b: &Built, wanted: impl Fn(&MeshFace) -> bool) -> u32 {
    b.faces.iter().find(|f| wanted(f)).map(|f| f.id).expect("the face")
}

#[test]
fn a_drilled_hole_is_a_hole_through_of_its_diameter() {
    let body = plate().boolean(&bore(5.0, -1.0, 12.0), 0).expect("drilled");
    let b = built(&body);
    let top = id_of(&b, |f| f.normal[2] > 0.99);
    // clicked on the upper half of the wall: drilled from the top
    let pick = Pick { at: [25.0, 15.0, 8.0], ..click(&b, wall_of(5.0)) };
    match feature_at(&b.mesh, &b.faces, pick, 1.0) {
        Ok(Found::Hole(h)) => {
            assert!((h.diameter - 10.0).abs() < 1e-3, "the hole is {} across, not 10", h.diameter);
            assert_eq!(h.reach, Reach::Through, "a hole through the plate");
            assert_eq!(h.entry, top, "drilled from the top, the end nearer the click");
            assert!((h.depth - 10.0).abs() < 1e-3 && (h.at[2] - 10.0).abs() < 1e-3 && h.inward[2] < -0.999, "{h:?}");
        }
        other => panic!("the drilled hole came back {other:?}"),
    }
}

#[test]
fn a_hole_of_flat_strips_is_one_hole_of_its_circle() {
    let mut wrong = Vec::new();
    for sides in [8, 12, 24, 64] {
        let body = plate().boolean(&prism(sides, 5.0, -1.0, 12.0), 0).expect("cut");
        let b = built(&body);
        match feature_at(&b.mesh, &b.faces, click(&b, wall_of(5.0)), 1.0) {
            // the corners of the strips lie on the circle of 5
            Ok(Found::Hole(h)) if (h.diameter - 10.0).abs() < 0.005 * 10.0 && h.reach == Reach::Through && h.faces.len() == sides => {}
            other => wrong.push(format!("{sides} strips: {other:?}")),
        }
    }
    assert!(wrong.is_empty(), "holes of strips not found whole:\n{}", wrong.join("\n"));
}

#[test]
fn a_blind_hole_takes_its_bottom_and_its_depth() {
    let body = plate().boolean(&bore(3.0, 4.0, 7.0), 0).expect("drilled 6 deep");
    let b = built(&body);
    let top = id_of(&b, |f| f.normal[2] > 0.99 && f.centroid.z > 9.9);
    let bottom = id_of(&b, |f| f.normal[2] > 0.99 && (f.centroid.z - 4.0).abs() < 1e-6);
    match feature_at(&b.mesh, &b.faces, click(&b, wall_of(3.0)), 1.0) {
        Ok(Found::Hole(h)) => {
            assert_eq!(h.reach, Reach::Blind, "{h:?}");
            assert_eq!(h.entry, top, "a blind hole opens through the top");
            assert!((h.depth - 6.0).abs() < 1e-3 && (h.diameter - 6.0).abs() < 1e-3, "{h:?}");
            assert!(h.faces.contains(&bottom), "the bottom of the hole is not taken with it: {:?}", h.faces);
        }
        other => panic!("the blind hole came back {other:?}"),
    }
}

/// The edge of the plate at x = 40, y = 30, by its id.
fn corner_edge(s: &Shape) -> u32 {
    s.edges_info().into_iter().find(|e| e.poly.iter().all(|p| (p[0] as f64 - 40.0).abs() < 1e-6 && (p[1] as f64 - 30.0).abs() < 1e-6)).map(|e| e.id).expect("the corner edge")
}

#[test]
fn a_rounded_edge_is_a_rounding_between_its_two_faces() {
    let p = plate();
    let body = p.fillet_edges(4.0, &[corner_edge(&p)]).expect("rounded");
    let b = built(&body);
    let (x, y) = (id_of(&b, |f| f.normal[0] > 0.999), id_of(&b, |f| f.normal[1] > 0.999));
    // the rounding is the face round the corner: neither a side nor a cap
    match feature_at(&b.mesh, &b.faces, click(&b, |f| f.centroid.x > 36.0 && f.centroid.y > 26.0 && f.normal[2].abs() < 0.1 && f.normal[0] < 0.999 && f.normal[1] < 0.999), 1.0) {
        Ok(Found::Round(r)) => {
            assert!((r.radius - 4.0).abs() < 1e-3, "rounded {}, not 4", r.radius);
            let mut sides = [r.sides.one, r.sides.other];
            sides.sort_unstable();
            let mut want = [x, y];
            want.sort_unstable();
            assert_eq!(sides, want, "the rounding stands between other faces");
        }
        other => panic!("the rounding came back {other:?}"),
    }
}

#[test]
fn a_cut_edge_is_a_chamfer_of_its_leg() {
    let p = plate();
    let body = p.chamfer_edges(3.0, &[corner_edge(&p)], &[], &[], &[]).expect("cut");
    let b = built(&body);
    match feature_at(&b.mesh, &b.faces, click(&b, |f| f.normal[0] > 0.1 && f.normal[1] > 0.1), 1.0) {
        Ok(Found::Chamfer(c)) => assert!((c.leg - 3.0).abs() < 1e-3, "cut back {}, not 3", c.leg),
        other => panic!("the chamfer came back {other:?}"),
    }
}

#[test]
fn a_boss_and_a_flat_face_are_no_feature() {
    let body = plate().boolean(&bore(4.0, 10.0, 5.0), 1).expect("a boss on top");
    let b = built(&body);
    let boss = feature_at(&b.mesh, &b.faces, click(&b, |f| f.normal[2].abs() < 0.1 && f.centroid.z > 10.0), 1.0);
    assert_eq!(boss, Err(Unfound::NoEntry), "a boss is not a hole");
    let side = feature_at(&b.mesh, &b.faces, click(&b, |f| f.normal[0] < -0.999), 1.0);
    assert_eq!(side, Err(Unfound::Other), "a side of the plate is no feature");
}

#[test]
fn a_measured_size_is_brought_to_what_was_drawn() {
    assert_eq!(rounded(2.96), 3.0);
    assert_eq!(rounded(12.04), 12.0);
    assert_eq!(rounded_bore(3.36), 3.4, "a clearance hole for M3");
    assert_eq!(rounded_bore(6.52), 6.6, "a clearance hole for M6");
    assert_eq!(rounded_bore(7.13), 7.1, "no bore near: a tenth");
}

/// A ROUNDING AND A HOLE COME IN AS A MESH, made a polyhedron: every flat run of triangles one face, the rounding a
/// handful of strips. The corners of the strips lie on the rounding's cylinder, and the click finds it whole.
#[test]
fn a_rounding_of_flat_strips_is_one_rounding() {
    let p = plate();
    let exact = p.fillet_edges(4.0, &[corner_edge(&p)]).expect("rounded").boolean(&bore(5.0, -1.0, 12.0), 0).expect("drilled");
    let mesh = exact.tessellate_merged(0.05).expect("a mesh").mesh;
    let body = Shape::from_mesh(&mesh).expect("a polyhedron");
    let b = built(&body);
    let strip = click(&b, |f| f.centroid.x > 36.0 && f.centroid.y > 26.0 && f.normal[2].abs() < 0.1 && f.normal[0] < 0.999 && f.normal[1] < 0.999);
    match feature_at(&b.mesh, &b.faces, strip, 1.0) {
        Ok(Found::Round(r)) => assert!((r.radius - 4.0).abs() < 0.02 && r.faces.len() > 1, "rounded {} over {} strips", r.radius, r.faces.len()),
        other => panic!("the rounding of strips came back {other:?}"),
    }
    match feature_at(&b.mesh, &b.faces, click(&b, wall_of(5.0)), 1.0) {
        Ok(Found::Hole(h)) => assert!((rounded_bore(h.diameter) - 10.0).abs() < 1e-9 && h.reach == Reach::Through, "{h:?}"),
        other => panic!("the hole of strips came back {other:?}"),
    }
}
