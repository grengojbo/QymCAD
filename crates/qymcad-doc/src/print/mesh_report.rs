//! THE HEALTH OF A MESH IN NUMBERS: what a slicer stumbles on before it slices. A hole leaves the inside open and the
//! slicer guesses where the wall is; an edge three triangles share has no inside to guess; a triangle wound inward
//! turns its face over; pieces that do not touch are printed apart. All of it is read as `qymcad_meshfit::prepare`
//! reads a mesh for recognition: corners closer than its weld are one, so a file that gives every triangle corners of
//! its own is whole where its triangles meet.

use qymcad_core::geom::Mesh;
use qymcad_meshfit::{prepare, weld_tolerance, Prepared, NO_NEIGHBOUR};

/// What a mesh is made of, as a slicer meets it.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshReport {
    /// The triangles of the file.
    pub triangles: usize,
    /// Triangles of no use: no area, two corners welded into one, a repeat of another, a corner that is no number.
    pub dropped: usize,
    /// The borders the mesh is open along, each a loop of edges no second triangle meets.
    pub holes: usize,
    /// The edges of those borders, all of them.
    pub open_edges: usize,
    /// The length of the longest border, mm: a hole of 3 edges closes itself, one round a whole side does not.
    pub widest_hole: f64,
    /// Edges three or more triangles share.
    pub non_manifold: usize,
    /// Triangles that were wound inward and are turned back.
    pub flipped: usize,
    /// Pieces of the mesh no triangle joins, each printed on its own; the hollows inside them are not counted here.
    pub pieces: usize,
    /// Closed pieces inside a solid, wound into it: cavities the file leaves on purpose, not triangles wound inward.
    pub hollows: usize,
    /// No hole and no edge of three: a solid a slicer reads as it is.
    pub closed: bool,
    /// The volume the mesh closes, mm^3, its triangles wound outward; on an open mesh a number of no meaning.
    pub volume: f64,
    /// The area of its triangles, mm^2.
    pub area: f64,
}

/// The health of `mesh`.
pub fn mesh_report(mesh: &Mesh) -> MeshReport {
    let p = prepare(mesh, weld_tolerance(mesh));
    let open_edges = p.holes.iter().map(Vec::len).sum();
    let widest_hole = p.holes.iter().map(|h| perimeter(&p, h)).fold(0.0, f64::max);
    MeshReport {
        triangles: mesh.tris.len(),
        dropped: p.dropped,
        holes: p.holes.len(),
        open_edges,
        widest_hole,
        non_manifold: p.non_manifold,
        flipped: p.flipped,
        pieces: pieces(&p) - p.hollows,
        hollows: p.hollows,
        closed: p.holes.is_empty() && p.non_manifold == 0,
        volume: volume(&p.mesh).abs(),
        area: (0..p.mesh.tris.len()).map(|t| p.mesh.tri_normal_area(t).1).sum(),
    }
}

/// The length of a border, its corners in the order it runs, back to the first.
fn perimeter(p: &Prepared, border: &[u32]) -> f64 {
    let at = |i: usize| p.mesh.verts[border[i % border.len()] as usize];
    (0..border.len())
        .map(|i| {
            let (a, b) = (at(i), at(i + 1));
            ((b.x - a.x).powi(2) + (b.y - a.y).powi(2) + (b.z - a.z).powi(2)).sqrt()
        })
        .sum()
}

/// The pieces the triangles fall into, joined across the sides they share.
fn pieces(p: &Prepared) -> usize {
    let mut root: Vec<usize> = (0..p.mesh.tris.len()).collect();
    fn find(root: &mut [usize], mut t: usize) -> usize {
        while root[t] != t {
            root[t] = root[root[t]];
            t = root[t];
        }
        t
    }
    for (t, across) in p.neighbours.iter().enumerate() {
        for &u in across.iter().filter(|&&u| u != NO_NEIGHBOUR) {
            let (a, b) = (find(&mut root, t), find(&mut root, u as usize));
            if a != b {
                root[a] = b;
            }
        }
    }
    (0..root.len()).filter(|&t| find(&mut root, t) == t).count()
}

/// The signed volume the triangles close, by the tetrahedra they make with the origin.
fn volume(m: &Mesh) -> f64 {
    (0..m.tris.len())
        .map(|t| {
            let [a, b, c] = m.triangle(t);
            (a.x * (b.y * c.z - b.z * c.y) - a.y * (b.x * c.z - b.z * c.x) + a.z * (b.x * c.y - b.y * c.x)) / 6.0
        })
        .sum()
}
