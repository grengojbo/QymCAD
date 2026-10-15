//! THE PRISM A FLAT FACE TOPS: a boss standing on a body, or a pocket sunk into it, found from its flat face.
//!
//! A boss of a solid from a file is faces and nothing more: its top, the walls round it, the body it stands on. To be
//! redrawn by a sketch it has to be known as one thing - which faces it is, how tall, which way. The walls are the faces
//! across the outer loop of the face; each has to stand square to it (a plane on edge, a cylinder along its normal), and
//! they all end on flat faces of one height facing the same way as the face: the base. Walls facing out of the loop
//! make a boss, walls facing into it a pocket. Loops inside the face are holes through the boss; one going deeper than
//! the boss is refused, as it could not be laid back by the sketch.
use super::{Id, Project};
use crate::errors::CoreError;
use crate::geom::{Mesh, MeshFace};

/// How far off square a wall may stand, and off flat the face, as the sine of the angle: 0.01 deg. Faces of a solid are
/// true to its tolerance; a wall drafted 1 deg is a slanted wall.
const SQUARE: f64 = 1.75e-4;
/// Two ends of edges closer than this, in mm, are one corner of a loop; two faces whose planes stand this close, one
/// height.
const NEAR: f64 = 1e-4;

/// Which way a prism stands from its face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrismKind {
    /// Standing out of the body: the face is its top.
    Boss,
    /// Sunk into the body: the face is its floor.
    Pocket,
}

/// A PRISM FOUND: its face, the faces round it and inside it that go with it, the faces it ends on, how tall it is and
/// which way it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct Prism {
    pub face: u32,
    pub walls: Vec<u32>,
    pub inner: Vec<u32>,
    pub base: Vec<u32>,
    /// The edges of the outer loop of the largest face of the base: the outline the prism is placed from.
    pub outline: Vec<u32>,
    pub height: f64,
    pub kind: PrismKind,
}

impl Prism {
    /// Every face that goes when the prism is taken off: its face, its walls, the faces inside it.
    pub fn faces(&self) -> Vec<u32> {
        std::iter::once(self.face).chain(self.walls.iter().copied()).chain(self.inner.iter().copied()).collect()
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = dot(v, v).sqrt().max(1e-300);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// The normals of the triangles of a face, of length one; a sliver has no direction and is left out.
fn normals(mesh: &Mesh, f: &MeshFace) -> Vec<[f64; 3]> {
    f.triangles
        .iter()
        .filter_map(|&t| {
            let [a, b, c] = mesh.triangle(t as usize);
            let (u, v) = ([b.x - a.x, b.y - a.y, b.z - a.z], [c.x - a.x, c.y - a.y, c.z - a.z]);
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            (dot(n, n) > 1e-24).then(|| unit(n))
        })
        .collect()
}

/// The corners of a face's triangles, read along `n` from `o`: the lowest and the highest.
fn reach_along(mesh: &Mesh, f: &MeshFace, o: [f64; 3], n: [f64; 3]) -> Option<Span> {
    let mut s: Option<Span> = None;
    for &t in &f.triangles {
        for v in mesh.triangle(t as usize) {
            let h = dot(sub([v.x, v.y, v.z], o), n);
            s = Some(match s {
                Some(Span { low, high }) => Span { low: low.min(h), high: high.max(h) },
                None => Span { low: h, high: h },
            });
        }
    }
    s
}

/// How far a face reaches along a direction.
#[derive(Clone, Copy)]
struct Span {
    low: f64,
    high: f64,
}

/// An edge of the face: its id, the face across it, and its ends.
struct Rim {
    edge: u32,
    across: u32,
    from: [f64; 3],
    to: [f64; 3],
}

/// The edges of a face chained into loops by their ends; an edge that closes on itself (a circle) is a loop alone.
fn loops(rims: Vec<Rim>) -> Vec<Vec<Rim>> {
    let close = |a: [f64; 3], b: [f64; 3]| dot(sub(a, b), sub(a, b)).sqrt() < NEAR;
    let mut left = rims;
    let mut out = Vec::new();
    while let Some(first) = left.pop() {
        let start = first.from;
        let mut end = first.to;
        let mut chain = vec![first];
        while !close(start, end) {
            let Some(k) = left.iter().position(|r| close(r.from, end) || close(r.to, end)) else { break };
            let r = left.remove(k);
            end = if close(r.from, end) { r.to } else { r.from };
            chain.push(r);
        }
        out.push(chain);
    }
    out
}

/// The area a loop encloses, seen along `n`: its ends taken in turn round the loop.
fn area_along(chain: &[Rim], n: [f64; 3]) -> f64 {
    let mut s = [0.0; 3];
    for r in chain {
        let c = [r.from[1] * r.to[2] - r.from[2] * r.to[1], r.from[2] * r.to[0] - r.from[0] * r.to[2], r.from[0] * r.to[1] - r.from[1] * r.to[0]];
        s = [s[0] + c[0], s[1] + c[1], s[2] + c[2]];
    }
    dot(s, n).abs() * 0.5
}

impl Project {
    /// THE PRISM FACE `face` OF BODY `body` TOPS, as the body stands now: refused in words when the face is not flat, when
    /// nothing stands on it as a prism, when a wall is slanted or its rim rounded, when the walls end at different
    /// heights, or when a hole inside goes deeper than the prism.
    pub fn prism_at(&self, body: Id, face: u32, kernel: &dyn crate::feature::Kernel) -> Result<Prism, CoreError> {
        let mi = self.mesh_index(body).ok_or(CoreError::SourceBodyNotBuilt)?;
        let (mesh, faces) = (&self.bodies[mi].mesh, &self.bodies[mi].faces);
        let by_id = |id: u32| faces.iter().find(|f| f.id == id);
        let top = by_id(face).ok_or(CoreError::FaceNotFound)?;
        let n = unit(top.normal);
        if normals(mesh, top).iter().any(|t| 1.0 - dot(*t, n) > SQUARE * SQUARE) {
            return Err(CoreError::PrismFaceNotFlat);
        }
        let o = [top.centroid.x, top.centroid.y, top.centroid.z];
        let pairs = kernel.edge_face_pairs(body);
        let geom: std::collections::HashMap<u32, Vec<[f64; 3]>> = kernel.body_edge_geometry(body).into_iter().map(|(id, poly, _)| (id, poly)).collect();
        let rims: Vec<Rim> = pairs
            .iter()
            .filter(|p| p.faces.contains(&face) && p.faces[0] != p.faces[1])
            .filter_map(|p| {
                let poly = geom.get(&p.edge)?;
                let across = if p.faces[0] == face { p.faces[1] } else { p.faces[0] };
                Some(Rim { edge: p.edge, across, from: *poly.first()?, to: *poly.last()? })
            })
            .collect();
        let mut chains = loops(rims);
        chains.sort_by(|a, b| area_along(b, n).total_cmp(&area_along(a, n)));
        let mut chains = chains.into_iter();
        let outer = chains.next().ok_or(CoreError::NotAPrism)?;
        let mut walls: Vec<u32> = outer.iter().map(|r| r.across).collect();
        walls.sort_unstable();
        walls.dedup();
        // every wall square to the face: a plane on edge, a cylinder along the normal
        let mut outward = 0.0;
        for r in &outer {
            let w = by_id(r.across).ok_or(CoreError::FaceNotFound)?;
            let ns = normals(mesh, w);
            if ns.iter().any(|t| dot(*t, n).abs() > SQUARE) {
                // a wall leaning off square all over is slanted; one bending into the face is a rounded or cut rim
                let off: f64 = ns.iter().map(|t| dot(*t, n).abs()).fold(0.0, f64::max);
                let flat_lean = ns.iter().all(|t| (dot(*t, n).abs() - off).abs() < SQUARE);
                return Err(if flat_lean && off < 0.5 { CoreError::PrismWallSlanted } else { CoreError::PrismRimRounded });
            }
            // which way the wall faces at its edge: out of the loop (a boss) or into it (a pocket)
            let mid = [(r.from[0] + r.to[0]) * 0.5, (r.from[1] + r.to[1]) * 0.5, (r.from[2] + r.to[2]) * 0.5];
            let wn = nearest_normal(mesh, w, mid).unwrap_or(w.normal);
            let away = sub(mid, o);
            let away = sub(away, [n[0] * dot(away, n), n[1] * dot(away, n), n[2] * dot(away, n)]);
            // weighed by the length of the edge: a long side tells more than a short one round a corner
            outward += dot(wn, away).signum() * dot(sub(r.to, r.from), sub(r.to, r.from)).sqrt().max(NEAR);
        }
        let kind = if outward > 0.0 { PrismKind::Boss } else { PrismKind::Pocket };
        // the base: the faces the walls end on, flat, facing as the face does, at one height
        let mut base = Vec::new();
        let mut height: Option<f64> = None;
        for p in pairs.iter().filter(|p| p.faces.iter().any(|f| walls.contains(f))) {
            for &f in p.faces.iter().filter(|f| **f != face && !walls.contains(f)) {
                let Some(b) = by_id(f) else { continue };
                if normals(mesh, b).iter().any(|t| 1.0 - dot(*t, n) > SQUARE * SQUARE) {
                    let facing = dot(unit(b.normal), n);
                    if facing.abs() < 1.0 - SQUARE {
                        return Err(CoreError::PrismRimRounded);
                    }
                    return Err(CoreError::NotAPrism);
                }
                let s = dot(sub([b.centroid.x, b.centroid.y, b.centroid.z], o), n);
                if height.is_some_and(|h| (h - s).abs() > NEAR) {
                    return Err(CoreError::PrismBaseUneven);
                }
                height = Some(s);
                if !base.contains(&f) {
                    base.push(f);
                }
            }
        }
        let s = height.ok_or(CoreError::NotAPrism)?;
        // a boss ends below its top, a pocket above its floor
        if (kind == PrismKind::Boss) != (s < -NEAR) || s.abs() <= NEAR {
            return Err(CoreError::NotAPrism);
        }
        // loops inside the face: holes through the prism, taken off with it while they end inside it
        let mut inner = Vec::new();
        for chain in chains {
            for r in chain {
                let Some(w) = by_id(r.across) else { continue };
                let reach = reach_along(mesh, w, o, n).ok_or(CoreError::FaceNotFound)?;
                if reach.low < s.min(0.0) - NEAR || reach.high > s.max(0.0) + NEAR {
                    return Err(CoreError::PrismHoleDeeper);
                }
                if !inner.contains(&r.across) {
                    inner.push(r.across);
                }
            }
        }
        // the outline of the base: the outer loop of its largest face, the loop the prism stands in left out
        let widest = base.iter().filter_map(|&b| by_id(b)).max_by(|a, b| a.area.total_cmp(&b.area)).map(|b| b.id);
        let outline = widest.map_or(Vec::new(), |b| {
            let rims: Vec<Rim> = pairs
                .iter()
                .filter(|p| p.faces.contains(&b) && p.faces[0] != p.faces[1])
                .filter_map(|p| {
                    let poly = geom.get(&p.edge)?;
                    let across = if p.faces[0] == b { p.faces[1] } else { p.faces[0] };
                    Some(Rim { edge: p.edge, across, from: *poly.first()?, to: *poly.last()? })
                })
                .collect();
            let mut chains = loops(rims);
            chains.sort_by(|a, b| area_along(b, n).total_cmp(&area_along(a, n)));
            chains.into_iter().next().map_or(Vec::new(), |c| c.into_iter().map(|r| r.edge).collect())
        });
        Ok(Prism { face, walls, inner, base, outline, height: s.abs(), kind })
    }
}

/// The normal of the triangle of face `f` lying nearest to `at`.
fn nearest_normal(mesh: &Mesh, f: &MeshFace, at: [f64; 3]) -> Option<[f64; 3]> {
    f.triangles
        .iter()
        .map(|&t| {
            let [a, b, c] = mesh.triangle(t as usize);
            let mid = [(a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0, (a.z + b.z + c.z) / 3.0];
            let (u, v) = ([b.x - a.x, b.y - a.y, b.z - a.z], [c.x - a.x, c.y - a.y, c.z - a.z]);
            let nn = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            Near { far: dot(sub(mid, at), sub(mid, at)), normal: nn }
        })
        .filter(|x| dot(x.normal, x.normal) > 1e-24)
        .min_by(|a, b| a.far.total_cmp(&b.far))
        .map(|x| unit(x.normal))
}

/// A triangle of a face: how far it lies from a point (squared), and its normal.
struct Near {
    far: f64,
    normal: [f64; 3],
}
