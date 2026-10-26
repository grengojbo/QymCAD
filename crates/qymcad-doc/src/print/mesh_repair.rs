//! A MESH MENDED FOR A SLICER, as far as it can be without guessing the shape: corners within the weld made one,
//! triangles of no use dropped and the rest wound outward (`qymcad_meshfit::prepare`), every hole of up to
//! `fill_up_to` edges closed by a fan from the middle of its border, pieces under `drop_under` mm^3 dropped, and the
//! whole made lighter within `simplify` mm. A wider hole is left open and counted: an open side of a scan closed by a
//! guess is a lid the person never drew.

use qymcad_core::geom::{Mesh, Point3};
use qymcad_core::model::Colours;
use qymcad_meshfit::{prepare, weld_tolerance};

/// What a repair is asked to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Repair {
    /// The pieces closing less than this many mm^3 are dropped; 0 keeps every piece.
    pub drop_under: f64,
    /// The holes of up to this many edges are closed; 0 closes none.
    pub fill_up_to: usize,
    /// The deviation, mm, the mesh is made lighter within; 0 leaves every triangle.
    pub simplify: f64,
}

/// The mended mesh and what was done to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Repaired {
    pub mesh: Mesh,
    /// For every triangle of `mesh`, the triangle of the mesh given it came of - a triangle closing a hole, the one
    /// across its border side - or none, when the mesh was made lighter.
    pub colours: Colours,
    pub closed_holes: usize,
    /// Holes wider than `fill_up_to`, left open.
    pub left_holes: usize,
    pub dropped_pieces: usize,
}

impl Repaired {
    /// The mended mesh as a piece of a mesh holds it: its triangles, the flat faces found on them as on a mesh brought
    /// in, and where their colours come from.
    pub fn into_piece(self) -> qymcad_core::model::PieceMesh {
        let faces = qymcad_meshfit::raw_faces(&self.mesh);
        qymcad_core::model::PieceMesh { mesh: self.mesh, faces, colours: self.colours }
    }
}

/// `mesh` mended as `how` asks.
pub fn mesh_repair(mesh: &Mesh, how: &Repair) -> Repaired {
    let p = prepare(mesh, weld_tolerance(mesh));
    let mut out = p.mesh.clone();
    let mut took: Vec<u32> = p.origin.clone();
    // which triangle holds a directed side, so a lid is wound against it
    let mut holder: std::collections::HashMap<[u32; 2], usize> = std::collections::HashMap::new();
    for (t, tri) in out.tris.iter().enumerate() {
        for k in 0..3 {
            holder.insert([tri[k], tri[(k + 1) % 3]], t);
        }
    }
    let mut closed_holes = 0;
    let mut left_holes = 0;
    for border in &p.holes {
        if border.len() < 3 || border.len() > how.fill_up_to {
            left_holes += 1;
            continue;
        }
        let n = border.len() as f64;
        let mid = border.iter().fold([0.0; 3], |s, &v| {
            let q = out.verts[v as usize];
            [s[0] + q.x / n, s[1] + q.y / n, s[2] + q.z / n]
        });
        let centre = out.verts.len() as u32;
        out.verts.push(Point3::new(mid[0], mid[1], mid[2]));
        for i in 0..border.len() {
            let a = border[i];
            let b = border[(i + 1) % border.len()];
            // the side runs a -> b in the triangle beside it: the lid runs it b -> a
            let (tri, beside) = match holder.get(&[a, b]) {
                Some(&t) => ([b, a, centre], t),
                None => ([a, b, centre], holder.get(&[b, a]).copied().unwrap_or(0)),
            };
            out.tris.push(tri);
            took.push(p.origin.get(beside).copied().unwrap_or(0));
        }
        closed_holes += 1;
    }
    let dropped_pieces = if how.drop_under > 0.0 { drop_crumbs(&mut out, &mut took, how.drop_under) } else { 0 };
    let colours = if how.simplify > 0.0 {
        out = qymcad_core::mesh_simplify::simplify(&out, how.simplify);
        Colours::Dropped
    } else {
        Colours::From(took)
    };
    Repaired { mesh: out, colours, closed_holes, left_holes, dropped_pieces }
}

/// Drops the pieces of `mesh` closing less than `under` mm^3, and their entries of `took`; returns how many went.
fn drop_crumbs(mesh: &mut Mesh, took: &mut Vec<u32>, under: f64) -> usize {
    // the pieces, joined across the sides two triangles share by their corners
    let mut root: Vec<usize> = (0..mesh.tris.len()).collect();
    fn find(root: &mut [usize], mut t: usize) -> usize {
        while root[t] != t {
            root[t] = root[root[t]];
            t = root[t];
        }
        t
    }
    let mut first: std::collections::HashMap<[u32; 2], usize> = std::collections::HashMap::new();
    for (t, tri) in mesh.tris.iter().enumerate() {
        for k in 0..3 {
            let side = [tri[k].min(tri[(k + 1) % 3]), tri[k].max(tri[(k + 1) % 3])];
            match first.get(&side) {
                Some(&u) => {
                    let (a, b) = (find(&mut root, t), find(&mut root, u));
                    if a != b {
                        root[a] = b;
                    }
                }
                None => {
                    first.insert(side, t);
                }
            }
        }
    }
    let mut volume: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    for t in 0..mesh.tris.len() {
        let [a, b, c] = mesh.triangle(t);
        let v = (a.x * (b.y * c.z - b.z * c.y) - a.y * (b.x * c.z - b.z * c.x) + a.z * (b.x * c.y - b.y * c.x)) / 6.0;
        *volume.entry(find(&mut root, t)).or_insert(0.0) += v;
    }
    // a hollow of the solid closes a volume below nothing, wound into the solid: it is no crumb, and dropped it would
    // fill the cavity the file left
    let small: std::collections::HashSet<usize> = volume.iter().filter(|(_, v)| (0.0..under).contains(*v)).map(|(&r, _)| r).collect();
    let keep: Vec<bool> = (0..mesh.tris.len()).map(|t| !small.contains(&find(&mut root, t))).collect();
    let mut k = 0;
    mesh.tris.retain(|_| {
        k += 1;
        keep[k - 1]
    });
    let mut k = 0;
    took.retain(|_| {
        k += 1;
        keep[k - 1]
    });
    // the corners of the pieces dropped go with them: left, they would still widen the box of the body
    let mut at = vec![u32::MAX; mesh.verts.len()];
    let mut verts = Vec::new();
    for tri in &mut mesh.tris {
        for v in tri.iter_mut() {
            if at[*v as usize] == u32::MAX {
                at[*v as usize] = verts.len() as u32;
                verts.push(mesh.verts[*v as usize]);
            }
            *v = at[*v as usize];
        }
    }
    mesh.verts = verts;
    small.len()
}
