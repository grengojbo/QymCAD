//! HOW THICK A BODY'S WALLS ARE, read on bodies whose walls are known: a hollow box of wall 0.6 is thinner than two
//! lines of a nozzle of 0.4 and is told where; of wall 2 it is not; a solid cube is as thick as its side; the same body
//! reads the same twice.
use qymcad_core::geom::{Mesh, Point3};
use qymcad_doc::print::checks::Verdict;
use qymcad_doc::print::thickness::{thickness, Walls};

/// A box: its sides, the corner it stands from, and which way its faces look.
struct Block {
    size: [f64; 3],
    at: [f64; 3],
    faces: Facing,
}

/// Which way a box's faces look: out of it, or into it - the walls of a hollow inside a body.
#[derive(Clone, Copy, PartialEq)]
enum Facing {
    Out,
    In,
}

fn cuboid(b: Block) -> Mesh {
    let Block { size, at, faces } = b;
    let c = |i: usize| Point3::new(at[0] + (i & 1) as f64 * size[0], at[1] + ((i >> 1) & 1) as f64 * size[1], at[2] + ((i >> 2) & 1) as f64 * size[2]);
    let mut tris = vec![[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    if faces == Facing::In {
        for t in &mut tris {
            t.swap(1, 2);
        }
    }
    Mesh { verts: (0..8).map(c).collect(), tris, ..Default::default() }
}

/// A box of 20 hollowed to walls of `wall` all round.
fn hollow(wall: f64) -> Mesh {
    let mut m = cuboid(Block { size: [20.0; 3], at: [0.0; 3], faces: Facing::Out });
    let inner = cuboid(Block { size: [20.0 - 2.0 * wall; 3], at: [wall; 3], faces: Facing::In });
    let shift = m.verts.len() as u32;
    m.verts.extend(inner.verts);
    m.tris.extend(inner.tris.into_iter().map(|t| t.map(|v| v + shift)));
    m
}

/// Two lines of a nozzle of 0.4.
const WALLS: Walls = Walls { least: 0.8, samples: 20_000 };

#[test]
fn a_box_of_thin_walls_is_told_where_it_is_thin() {
    let t = thickness(&hollow(0.6), &WALLS);
    assert_eq!(t.verdict, Verdict::Fail, "{t:?}");
    assert!((t.least - 0.6).abs() < 0.05, "the walls of 0.6 read {} thin", t.least);
    assert!(!t.places.is_empty() && t.places.len() <= 10, "{} places told", t.places.len());
    for p in &t.places {
        assert!((p.thickness - 0.6).abs() < 0.05, "a place told {} thick on walls of 0.6", p.thickness);
        let on_a_wall = p.at.iter().any(|&c| c.abs() < 1e-6 || (c - 20.0).abs() < 1e-6 || (c - 0.6).abs() < 1e-6 || (c - 19.4).abs() < 1e-6);
        assert!(on_a_wall, "a place {:?} lies on no wall", p.at);
    }
}

#[test]
fn a_box_of_walls_of_two_holds() {
    let t = thickness(&hollow(2.0), &WALLS);
    assert_eq!(t.verdict, Verdict::Pass, "{t:?}");
    assert!((t.least - 2.0).abs() < 0.05, "the walls of 2 read {} thin", t.least);
    assert!(t.places.is_empty(), "a place told thin on walls of 2: {:?}", t.places);
}

#[test]
fn a_solid_cube_is_as_thick_as_its_side() {
    let t = thickness(&cuboid(Block { size: [10.0; 3], at: [0.0; 3], faces: Facing::Out }), &WALLS);
    assert_eq!(t.verdict, Verdict::Pass, "{t:?}");
    // a ray from near an edge meets the side across the corner first: less than 10, never more
    assert!(t.least > 0.0 && t.least <= 10.0 + 1e-9, "{t:?}");
    assert!(t.samples >= 10_000, "{} points read on a cube", t.samples);
}

#[test]
fn the_same_body_reads_the_same_twice() {
    let m = hollow(0.6);
    assert_eq!(thickness(&m, &WALLS), thickness(&m, &WALLS), "two reads of one body differ");
}

/// A ball of `bands` x `bands / 2` quads, two triangles each: `bands * bands` triangles.
fn ball(bands: u32) -> Mesh {
    let (around, up) = (bands, bands / 2);
    let mut m = Mesh::default();
    for j in 0..=up {
        let v = std::f64::consts::PI * f64::from(j) / f64::from(up);
        for i in 0..around {
            let u = std::f64::consts::TAU * f64::from(i) / f64::from(around);
            m.verts.push(Point3::new(20.0 * v.sin() * u.cos(), 20.0 * v.sin() * u.sin(), 20.0 * v.cos()));
        }
    }
    for j in 0..up {
        for i in 0..around {
            let (a, b) = (j * around + i, j * around + (i + 1) % around);
            let (c, d) = (a + around, b + around);
            m.tris.push([a, c, b]);
            m.tris.push([b, c, d]);
        }
    }
    m
}

#[test]
#[ignore = "a measurement of time: run by hand in release - cargo test --release -p qymcad-doc --test thickness -- --ignored"]
fn a_mesh_of_a_million_triangles_is_read_within_two_seconds() {
    if cfg!(debug_assertions) {
        eprintln!("PASSED OVER: the budget of time holds for a release build only");
        return;
    }
    let m = ball(1000);
    assert!(m.tris.len() >= 1_000_000, "{} triangles", m.tris.len());
    let start = std::time::Instant::now();
    let t = thickness(&m, &WALLS);
    let took = start.elapsed();
    eprintln!("{} triangles, {} points: {took:?}, least {}", m.tris.len(), t.samples, t.least);
    assert!((t.least - 40.0).abs() < 1.0 || t.least < 40.0, "{t:?}");
    assert!(took < std::time::Duration::from_secs(2), "a mesh of a million triangles took {took:?}");
}
