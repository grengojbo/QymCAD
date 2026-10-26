//! A HOLLOW CLOSED INSIDE A SOLID IS WOUND INTO THE SOLID, not out of itself: a cube of 10 with a hollow cube of 2 in
//! its middle - a cavity a print program leaves on purpose - keeps the hollow's triangles facing into it. Turned out
//! of itself, the hollow becomes a solid crumb inside the solid, and the body reads 1 000 + 8 instead of 1 000 - 8.
//! A hollow a file wound the wrong way is turned; one that is not inside anything faces out of itself as before.

use qymcad_core::geom::{Mesh, Point3};
use qymcad_meshfit::{prepare, weld_tolerance};

/// A box: its side, the corner it stands from, and which way its faces look.
struct Block {
    side: f64,
    at: [f64; 3],
    faces: Facing,
}

/// Which way the faces of a box look: out of it, or into it.
#[derive(Clone, Copy, PartialEq)]
enum Facing {
    Out,
    In,
}

fn cube(b: Block) -> Mesh {
    let Block { side, at, faces } = b;
    let c = |i: usize| Point3::new(at[0] + (i & 1) as f64 * side, at[1] + ((i >> 1) & 1) as f64 * side, at[2] + ((i >> 2) & 1) as f64 * side);
    let mut tris = vec![[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    if faces == Facing::In {
        for t in &mut tris {
            t.swap(1, 2);
        }
    }
    Mesh { verts: (0..8).map(c).collect(), tris, ..Default::default() }
}

fn joined(parts: Vec<Mesh>) -> Mesh {
    let mut out = Mesh::default();
    for m in parts {
        let shift = out.verts.len() as u32;
        out.verts.extend(m.verts);
        out.tris.extend(m.tris.into_iter().map(|t| t.map(|v| v + shift)));
    }
    out
}

/// The volume the triangles close, signed by their winding.
fn volume(m: &Mesh) -> f64 {
    m.tris
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| m.verts[i as usize]);
            (a.x * (b.y * c.z - b.z * c.y) - a.y * (b.x * c.z - b.z * c.x) + a.z * (b.x * c.y - b.y * c.x)) / 6.0
        })
        .sum()
}

fn hollow_cube(hollow: Facing) -> Mesh {
    joined(vec![cube(Block { side: 10.0, at: [0.0; 3], faces: Facing::Out }), cube(Block { side: 2.0, at: [4.0; 3], faces: hollow })])
}

#[test]
fn a_hollow_wound_into_the_solid_is_left_as_it_is() {
    let m = hollow_cube(Facing::In);
    let p = prepare(&m, weld_tolerance(&m));
    assert_eq!(p.flipped, 0, "the triangles of a hollow facing into the solid were turned");
    assert!((volume(&p.mesh) - 992.0).abs() < 1e-9, "the solid closes {} mm^3, not 1 000 - 8", volume(&p.mesh));
}

#[test]
fn a_hollow_wound_out_of_itself_is_turned_into_the_solid() {
    let m = hollow_cube(Facing::Out);
    let p = prepare(&m, weld_tolerance(&m));
    assert_eq!(p.flipped, 12, "the hollow's 12 triangles facing out of it were not turned");
    assert!((volume(&p.mesh) - 992.0).abs() < 1e-9, "the solid closes {} mm^3, not 1 000 - 8", volume(&p.mesh));
}

#[test]
fn two_solids_apart_both_face_out() {
    let m = joined(vec![cube(Block { side: 10.0, at: [0.0; 3], faces: Facing::Out }), cube(Block { side: 2.0, at: [20.0, 0.0, 0.0], faces: Facing::In })]);
    let p = prepare(&m, weld_tolerance(&m));
    assert_eq!(p.flipped, 12, "a cube beside the other, wound inward, was not turned out");
    assert!((volume(&p.mesh) - 1008.0).abs() < 1e-9, "two cubes apart close {} mm^3, not 1 000 + 8", volume(&p.mesh));
}
