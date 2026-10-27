//! MANY SEAMS ARE MENDED IN THE TIME OF ONE: a flat triangle closing a T-junction is mended by splitting its partner,
//! and the sides of the mesh were gathered again for every one of them - on a printed lamp module of 161 048
//! triangles `mesh_report` took 72.5 s, past the 60 s a client waits, and the server answered nothing else meanwhile.
//! A strip of 4 000 such seams is mended whole within a budget a single gathering keeps.

use qymcad_core::geom::{Mesh, Point3};
use qymcad_core::time_budget::{cost_of, holds, over, Doer};
use qymcad_meshfit::{prepare, weld_tolerance};

/// A strip of `cells` T-junctions along x = 0: on the left one triangle on the side from (0, k) to (0, k + 1), on the
/// right two triangles meeting at its middle (0, k + 0.5), and between them the flat triangle closing the seam.
fn seams(cells: u32) -> Mesh {
    let mut m = Mesh::default();
    for k in 0..cells {
        let y = f64::from(k);
        let at = m.verts.len() as u32;
        m.verts.extend([
            Point3::new(0.0, y, 0.0),
            Point3::new(0.0, y + 1.0, 0.0),
            Point3::new(0.0, y + 0.5, 0.0),
            Point3::new(-1.0, y + 0.5, 0.0),
            Point3::new(1.0, y, 0.0),
            Point3::new(1.0, y + 1.0, 0.0),
        ]);
        let [a, b, mid, left, low, high] = [at, at + 1, at + 2, at + 3, at + 4, at + 5];
        m.tris.extend([[a, b, left], [a, low, mid], [mid, high, b], [b, mid, a]]);
    }
    m
}

#[test]
fn a_strip_of_four_thousand_seams_is_mended_in_time() {
    let m = seams(4000);
    let read = cost_of(Doer::Thread, || prepare(&m, weld_tolerance(&m)));
    assert!(read.out.slivers.is_empty(), "{} seams left as slivers", read.out.slivers.len());
    // every seam mended: the flat one gone, its partner split in two - as many triangles as were given
    assert_eq!(read.out.mesh.tris.len(), m.tris.len(), "the seams were not mended one for one");
    let budget = std::time::Duration::from_secs(3);
    assert!(holds(read.cost, budget), "{}", over("mending 4 000 seams", read.cost, budget));
}
