//! EVERY HOLE, ROUNDING AND CHAMFER OF A BODY IS FOUND AT ONCE, each of them once, no face in two of them. The bodies
//! come from our kernel, so what they hold is known: a plate 60 x 40 x 10 drilled 5 and 6 through and 4 blind, one
//! corner rounded 4 and another cut back 3; a block whose four upright edges are rounded 4 and whose top edge is then
//! rounded 2 all round - the corners where the 2 runs round the 4 are faces of neither, and go with the rounding laid
//! last; the same plate cut by prisms of 24 and 64 sides, the holes a mesh draws with flat strips; a plain block.

use qymcad_core::geom::{Built, MeshFace};
use qymcad_kernel::specimen::{self, Specimen};
use qymcad_kernel::Shape;
use qymcad_meshfit::feature::{features_of, Found, Reach};

/// A block 60 x 40 x 10 from the origin.
fn block() -> Shape {
    specimen::extrude(&[0.0, 0.0, 60.0, 0.0, 60.0, 40.0, 0.0, 40.0], 10.0).expect("a block")
}

/// `s` moved by `by`.
fn placed(s: &Shape, by: [f64; 3]) -> Shape {
    specimen::placed(s, &[1.0, 0.0, 0.0, by[0], 0.0, 1.0, 0.0, by[1], 0.0, 0.0, 1.0, by[2]]).expect("placed")
}

/// A drill of radius `r` at (x, y), from z = `from` up `h`.
struct Drill {
    x: f64,
    y: f64,
    r: f64,
    from: f64,
    h: f64,
}

fn bore(d: Drill) -> Shape {
    placed(&specimen::cylinder(d.r, d.h).expect("a cylinder"), [d.x, d.y, d.from])
}

/// A prism of `sides` round (x, y), its corners `r` from the axis, through the plate.
fn prism(sides: usize, d: Drill) -> Shape {
    let xy: Vec<f64> = (0..sides)
        .flat_map(|k| {
            let a = std::f64::consts::TAU * k as f64 / sides as f64;
            [d.r * a.cos(), d.r * a.sin()]
        })
        .collect();
    placed(&specimen::extrude(&xy, d.h).expect("a prism"), [d.x, d.y, d.from])
}

/// The edge of `s` upright at (x, y).
fn upright(s: &Shape, (x, y): (f64, f64)) -> u32 {
    s.edges_info().into_iter().find(|e| e.poly.iter().all(|p| (p[0] as f64 - x).abs() < 1e-6 && (p[1] as f64 - y).abs() < 1e-6)).map(|e| e.id).expect("the upright edge")
}

fn plate() -> Shape {
    let p = block();
    let p = p.filleted(4.0, &[upright(&p, (60.0, 40.0))]).expect("rounded");
    let p = p.chamfered(3.0, &[upright(&p, (0.0, 0.0))]).expect("cut");
    [
        Drill { x: 15.0, y: 20.0, r: 3.0, from: -1.0, h: 12.0 },
        Drill { x: 30.0, y: 10.0, r: 2.5, from: -1.0, h: 12.0 },
        Drill { x: 45.0, y: 20.0, r: 2.0, from: 5.0, h: 6.0 },
    ]
    .into_iter()
    .fold(p, |p, d| p.combined(&bore(d), 0).expect("drilled"))
}

fn chained() -> Shape {
    let p = block();
    let ups: Vec<u32> = [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)].iter().map(|&at| upright(&p, at)).collect();
    let p = p.filleted(4.0, &ups).expect("rounded upright");
    let top: Vec<u32> = p.edges_info().into_iter().filter(|e| e.poly.iter().all(|q| (q[2] as f64 - 10.0).abs() < 1e-6)).map(|e| e.id).collect();
    let p = p.filleted(2.0, &top).expect("rounded round the top");
    p.combined(&bore(Drill { x: 15.0, y: 20.0, r: 3.0, from: -1.0, h: 12.0 }), 0).expect("drilled")
}

fn built(s: &Shape) -> Built {
    specimen::mesh_merged(s, 0.01).expect("a body")
}

/// What a body is found to hold, told plainly: the kind and its size, sorted.
fn told(found: &[Found]) -> Vec<String> {
    let mut out: Vec<String> = found
        .iter()
        .map(|f| match f {
            Found::Hole(h) => format!("hole {:.1} {}", h.diameter, if h.reach == Reach::Through { "through" } else { "blind" }),
            Found::Round(r) => format!("round {:.1}", r.radius),
            Found::Chamfer(c) => format!("chamfer {:.1}", c.leg),
        })
        .collect();
    out.sort();
    out
}

/// No face is in two features.
fn apart(found: &[Found]) -> bool {
    let all: Vec<u32> = found.iter().flat_map(|f| f.faces().to_vec()).collect();
    let mut once = all.clone();
    once.sort_unstable();
    once.dedup();
    once.len() == all.len()
}

/// Whether every triangle of a face turns the same way: a flat face.
fn flat(b: &Built, f: &MeshFace) -> bool {
    let n = |t: u32| {
        let [a, c, d] = b.mesh.tris[t as usize].map(|i| b.mesh.verts[i as usize]);
        let (u, v) = ([c.x - a.x, c.y - a.y, c.z - a.z], [d.x - a.x, d.y - a.y, d.z - a.z]);
        let m = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let l = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
        [m[0] / l, m[1] / l, m[2] / l]
    };
    let first = f.triangles.first().map(|&t| n(t));
    f.triangles.iter().all(|&t| {
        first.is_some_and(|a| {
            let m = n(t);
            a[0] * m[0] + a[1] * m[1] + a[2] * m[2] > 0.9999
        })
    })
}

#[test]
fn a_plate_gives_its_three_holes_its_rounding_and_its_chamfer() {
    let b = built(&plate());
    let all = features_of(&b.mesh, &b.faces, 1.0);
    assert_eq!(told(&all.found), ["chamfer 3.0", "hole 4.0 blind", "hole 5.0 through", "hole 6.0 through", "round 4.0"]);
    assert!(apart(&all.found), "a face is in two features: {:?}", all.found);
    assert!(all.unfound.is_empty(), "curved faces left: {:?}", all.unfound);
}

#[test]
fn the_corners_of_a_chain_of_roundings_go_with_the_rounding_laid_last() {
    let b = built(&chained());
    let all = features_of(&b.mesh, &b.faces, 1.0);
    assert_eq!(told(&all.found), ["hole 6.0 through", "round 2.0", "round 2.0", "round 2.0", "round 2.0", "round 4.0", "round 4.0", "round 4.0", "round 4.0"]);
    assert!(apart(&all.found), "a face is in two features: {:?}", all.found);
    assert!(all.unfound.is_empty(), "curved faces left: {:?}", all.unfound);
    // every curved face of the block is in a feature: the four corners went with the roundings of 2
    let taken: Vec<u32> = all.found.iter().flat_map(|f| f.faces().to_vec()).collect();
    let curved: Vec<u32> = b.faces.iter().filter(|f| !flat(&b, f)).map(|f| f.id).collect();
    assert!(curved.iter().all(|id| taken.contains(id)), "curved faces {curved:?}, taken {taken:?}");
    let small: usize = all.found.iter().filter_map(|f| if let Found::Round(r) = f { (r.radius < 3.0).then_some(r.faces.len()) } else { None }).sum();
    assert_eq!(small, 8, "the roundings of 2 take their four faces and the four corners");
}

#[test]
fn holes_drawn_with_flat_strips_are_each_one_hole() {
    for sides in [24, 64] {
        let p = block();
        let p = [Drill { x: 15.0, y: 20.0, r: 3.0, from: -1.0, h: 12.0 }, Drill { x: 40.0, y: 20.0, r: 5.0, from: -1.0, h: 12.0 }]
            .into_iter()
            .fold(p, |p, d| p.combined(&prism(sides, d), 0).expect("cut"));
        let b = built(&p);
        let all = features_of(&b.mesh, &b.faces, 1.0);
        let holes: Vec<String> = told(&all.found);
        assert_eq!(holes.len(), 2, "{sides} sides: {holes:?}");
        assert!(holes.iter().all(|h| h.starts_with("hole") && h.ends_with("through")), "{sides} sides: {holes:?}");
        assert!(apart(&all.found), "{sides} sides: a face is in two features");
    }
}

#[test]
fn a_plain_block_holds_nothing() {
    let b = built(&block());
    let all = features_of(&b.mesh, &b.faces, 1.0);
    assert!(all.found.is_empty(), "{:?}", all.found);
    assert!(all.unfound.is_empty());
}
