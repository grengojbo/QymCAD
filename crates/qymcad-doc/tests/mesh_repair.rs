//! A MESH MENDED FOR A SLICER: its corners welded, its triangles wound outward, its small holes closed, its crumbs
//! dropped - each read back by `mesh_report` on a mesh whose answer is known. A hole too wide to close by guessing is
//! left and told.
use qymcad_core::geom::{Mesh, Point3};
use qymcad_core::model::Colours;
use qymcad_doc::print::mesh_repair::{mesh_repair, Repair};
use qymcad_doc::print::mesh_report::mesh_report;

/// A cube of side `side` at `at`, its eight corners shared by its twelve triangles, wound outward.
fn cube(side: f64, at: [f64; 3]) -> Mesh {
    let c = |i: usize| Point3::new(at[0] + (i & 1) as f64 * side, at[1] + ((i >> 1) & 1) as f64 * side, at[2] + ((i >> 2) & 1) as f64 * side);
    let tris = vec![[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    Mesh { verts: (0..8).map(c).collect(), tris, ..Default::default() }
}

/// The meshes `parts` as one.
fn joined(parts: Vec<Mesh>) -> Mesh {
    let mut out = Mesh::default();
    for m in parts {
        let shift = out.verts.len() as u32;
        out.verts.extend(m.verts);
        out.tris.extend(m.tris.into_iter().map(|t| t.map(|v| v + shift)));
    }
    out
}

/// A tube of radius 10 and height 20 round z, `around` sides, open at both ends: two holes of `around` edges each.
fn tube(around: u32) -> Mesh {
    let mut m = Mesh::default();
    for k in 0..around {
        let a = std::f64::consts::TAU * f64::from(k) / f64::from(around);
        m.verts.push(Point3::new(10.0 * a.cos(), 10.0 * a.sin(), 0.0));
        m.verts.push(Point3::new(10.0 * a.cos(), 10.0 * a.sin(), 20.0));
    }
    for k in 0..around {
        let (b0, t0, b1, t1) = (2 * k, 2 * k + 1, 2 * ((k + 1) % around), 2 * ((k + 1) % around) + 1);
        m.tris.push([b0, b1, t1]);
        m.tris.push([b0, t1, t0]);
    }
    m
}

const AS_IS: Repair = Repair { fill_up_to: 64, drop_under: 0.0, simplify: 0.0 };

#[test]
fn a_cube_short_of_a_triangle_is_closed() {
    let mut m = cube(10.0, [0.0; 3]);
    m.tris.remove(2);
    let r = mesh_repair(&m, &AS_IS);
    assert_eq!(r.closed_holes, 1, "the hole of three edges was not closed");
    let after = mesh_report(&r.mesh);
    assert!(after.closed, "{after:?}");
    assert!((after.volume - 1000.0).abs() < 1e-9, "{after:?}");
    match &r.colours {
        Colours::From(took) => assert_eq!(took.len(), r.mesh.tris.len(), "a triangle of the mended mesh takes no colour"),
        Colours::Dropped => panic!("colours dropped where no triangle was made lighter"),
    }
}

#[test]
fn a_cube_short_of_a_side_is_closed_flat() {
    let mut m = cube(10.0, [0.0; 3]);
    m.tris.drain(2..4);
    let r = mesh_repair(&m, &AS_IS);
    let after = mesh_report(&r.mesh);
    assert!(after.closed, "{after:?}");
    // the side closed in its own plane: the fan's middle stands at the middle of the square
    assert!((after.volume - 1000.0).abs() < 1e-9, "{after:?}");
}

#[test]
fn a_hole_wider_than_asked_is_left_and_told() {
    let r = mesh_repair(&tube(100), &AS_IS);
    assert_eq!(r.closed_holes, 0, "a hole of 100 edges was closed past the 64 asked");
    assert_eq!(r.left_holes, 2, "the two ends of the tube are not told as left open");
    let wide = mesh_repair(&tube(100), &Repair { fill_up_to: 100, ..AS_IS });
    assert_eq!(wide.closed_holes, 2, "the ends of 100 edges are not closed when asked for 100");
    assert!(mesh_report(&wide.mesh).closed);
}

#[test]
fn a_triangle_wound_inward_is_turned_back() {
    let mut m = cube(10.0, [0.0; 3]);
    m.tris[4] = [m.tris[4][0], m.tris[4][2], m.tris[4][1]];
    let r = mesh_repair(&m, &AS_IS);
    assert_eq!(mesh_report(&r.mesh).flipped, 0, "a triangle stays wound inward");
}

#[test]
fn crumbs_under_the_volume_asked_are_dropped() {
    let m = joined(vec![cube(10.0, [0.0; 3]), cube(0.5, [30.0, 0.0, 0.0])]);
    let r = mesh_repair(&m, &Repair { drop_under: 1.0, ..AS_IS });
    assert_eq!(r.dropped_pieces, 1, "the crumb of 0.125 mm^3 was not dropped");
    let after = mesh_report(&r.mesh);
    assert_eq!(after.pieces, 1, "{after:?}");
    assert!((after.volume - 1000.0).abs() < 1e-9, "{after:?}");
    // the crumb's corners go with it: the box of the mended mesh is the cube's
    let far = r.mesh.verts.iter().map(|v| v.x).fold(f64::MIN, f64::max);
    assert!((far - 10.0).abs() < 1e-9, "a corner of the crumb dropped stays at x = {far}");
    // nothing asked, nothing dropped
    assert_eq!(mesh_repair(&m, &AS_IS).dropped_pieces, 0);
}

#[test]
fn a_mesh_made_lighter_drops_its_colours() {
    // closed first: a border stays as it is when a mesh is made lighter, and a tube open at both ends is all border
    let r = mesh_repair(&tube(100), &Repair { fill_up_to: 100, simplify: 0.5, ..AS_IS });
    assert!(r.mesh.tris.len() < 400, "a closed tube of 400 triangles is no lighter: {}", r.mesh.tris.len());
    assert!(mesh_report(&r.mesh).closed, "the tube made lighter is open");
    assert_eq!(r.colours, Colours::Dropped, "the colours of triangles that are no more are kept");
}

/// A HOLLOW IS NO CRUMB: a cube of 10 with a hollow of 8 mm^3 in it, mended with crumbs under 10 mm^3 dropped, keeps its
/// hollow facing into the solid - the mesh as it came, 1 000 - 8.
#[test]
fn a_hollow_is_kept_as_it_is() {
    let mut inner = cube(2.0, [4.0; 3]);
    for t in &mut inner.tris {
        t.swap(1, 2);
    }
    let m = joined(vec![cube(10.0, [0.0; 3]), inner]);
    let r = mesh_repair(&m, &Repair { drop_under: 10.0, ..AS_IS });
    assert_eq!(r.dropped_pieces, 0, "the hollow was dropped as a crumb");
    let after = mesh_report(&r.mesh);
    assert_eq!(after.hollows, 1, "{after:?}");
    assert!((after.volume - 992.0).abs() < 1e-9, "{after:?}");
}
