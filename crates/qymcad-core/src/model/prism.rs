//! THE PRISM A FLAT FACE TOPS: a boss standing on a body, or a pocket sunk into it, found from its flat face.
//!
//! A boss of a solid from a file is faces and nothing more: its top, the walls round it, the body it stands on. To be
//! redrawn by a sketch it has to be known as one thing - which faces it is, how tall, which way. The walls are the faces
//! across the outer loop of the face; each has to stand square to it (a plane on edge, a cylinder along its normal), and
//! they all end on flat faces of one height facing the same way as the face: the base. Walls facing out of the loop
//! make a boss, walls facing into it a pocket. Loops inside the face are holes through the boss; one going deeper than
//! the boss is refused, as it could not be laid back by the sketch.
//!
//! A BLOCK SEEN FROM ONE OF ITS ENDS is a prism too: its walls run from the face to flat faces facing the other way - the
//! far end of the body - and the body stands on nothing (a stock). A plate is the stock its top or its bottom is clicked
//! on, a box primitive the stock of any face. What stands on its ends - a boss on the top, a pocket in it, a hole through -
//! stays: loops inside the face are drawn in the sketch and are not taken off with it, but a hole running through is.
//! Something standing on the walls or on the far end, the sketch could not lay back, and is refused in words.
//!
//! A RIM ROUNDED OR CUT - at the face, between it and the walls, or at the base - is a strip between them, and corner
//! pieces where strips meet. It is found by walking from the face across the faces that are neither square to it nor
//! flat with it until square walls are met, and from the walls down to flat faces the same way. A rim is taken whole: the
//! same rounding or chamfer of one size all round, told by how far its strips reach along the normal. The outline of the
//! prism is then the edges where the walls meet what is above them - the face itself is smaller than the boss by the
//! rounding.
use super::{Id, Project};
use crate::errors::CoreError;
use crate::geom::{Mesh, MeshFace, Point2};

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
    /// The body itself from one of its ends: the walls run to the far end, which faces the other way.
    Stock,
}

/// How a rim is made: rounded, or cut flat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RimCut {
    Round,
    Chamfer,
}

/// A RIM ROUNDED OR CUT ALL ROUND: how, and how big - the radius of a rounding, the leg of a chamfer.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RimBlend {
    pub cut: RimCut,
    pub size: f64,
}

/// THE RIMS OF A PRISM: at its face (between the face and the walls) and at its base (between the walls and the face
/// they stand on); none where the edge is sharp.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rims {
    pub at_face: Option<RimBlend>,
    pub at_base: Option<RimBlend>,
}

/// A PRISM FOUND: its face, the faces round it and inside it that go with it, the faces it ends on, how tall it is and
/// which way it stands, its rims, and the edges its outline is copied from.
#[derive(Clone, Debug, PartialEq)]
pub struct Prism {
    pub face: u32,
    pub walls: Vec<u32>,
    pub inner: Vec<u32>,
    /// The strips and corner pieces of its rims.
    pub rim_faces: Vec<u32>,
    pub base: Vec<u32>,
    /// The edges of the outer loop of the largest face of the base: the outline the prism is placed from.
    pub outline: Vec<u32>,
    /// The edges whose plan is the outline of the prism - where the walls meet what is above them - and the edges of the
    /// loops inside its face.
    pub profile: Vec<u32>,
    pub height: f64,
    pub kind: PrismKind,
    pub rims: Rims,
}

impl Prism {
    /// Every face that goes when the prism is taken off: its face, its walls, the faces inside it, its rims.
    pub fn faces(&self) -> Vec<u32> {
        std::iter::once(self.face).chain(self.walls.iter().copied()).chain(self.inner.iter().copied()).chain(self.rim_faces.iter().copied()).collect()
    }
}

/// A direction or a point in space, as the mesh and the kernel hand them.
trait V3 {
    fn dot(self, o: Self) -> f64;
    fn minus(self, o: Self) -> Self;
}

impl V3 for [f64; 3] {
    fn dot(self, o: Self) -> f64 {
        self[0] * o[0] + self[1] * o[1] + self[2] * o[2]
    }

    fn minus(self, o: Self) -> Self {
        [self[0] - o[0], self[1] - o[1], self[2] - o[2]]
    }
}

/// A line along a normal from a point: how far a point or a face reaches is read along it.
#[derive(Clone, Copy)]
struct Along {
    from: [f64; 3],
    n: [f64; 3],
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = v.dot(v).sqrt().max(1e-300);
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
            (n.dot(n) > 1e-24).then(|| unit(n))
        })
        .collect()
}

/// The corners of a face's triangles, read along `n` from `o`: the lowest and the highest.
fn reach_along(mesh: &Mesh, f: &MeshFace, along: Along) -> Option<Span> {
    let Along { from: o, n } = along;
    let mut s: Option<Span> = None;
    for &t in &f.triangles {
        for v in mesh.triangle(t as usize) {
            let h = ([v.x, v.y, v.z].minus(o)).dot(n);
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
    let close = |a: [f64; 3], b: [f64; 3]| a.minus(b).dot(a.minus(b)).sqrt() < NEAR;
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
    s.dot(n).abs() * 0.5
}

/// The faces a walk across a body reached, and the faces it stopped at.
struct Walked {
    over: Vec<u32>,
    stopped: Vec<u32>,
}

/// Where a walk across the faces starts, and the faces it keeps off.
struct Walk<'a> {
    from: &'a [u32],
    kept: &'a [u32],
}

/// WALK ACROSS THE FACES FROM `from`: the faces `goes` lets through are walked over, those it does not are where the walk
/// stops; the faces in `kept` are neither.
fn walk(pairs: &[crate::feature::EdgeFaces], w: Walk, goes: impl Fn(u32) -> bool) -> Walked {
    let Walk { from, kept } = w;
    let mut over: Vec<u32> = Vec::new();
    let mut stopped: Vec<u32> = Vec::new();
    let mut todo: Vec<u32> = from.to_vec();
    while let Some(f) = todo.pop() {
        if kept.contains(&f) || over.contains(&f) || stopped.contains(&f) {
            continue;
        }
        if !goes(f) {
            stopped.push(f);
            continue;
        }
        over.push(f);
        for p in pairs.iter().filter(|p| p.faces.contains(&f) && p.faces[0] != p.faces[1]) {
            todo.push(if p.faces[0] == f { p.faces[1] } else { p.faces[0] });
        }
    }
    Walked { over, stopped }
}

impl Project {
    /// THE PRISM FACE `face` OF BODY `body` TOPS, as the body stands now - a boss, a pocket, or the body itself from one of
    /// its ends: refused in words when the face is not flat, when nothing stands on it as a prism, when something stands on
    /// the walls or the far end of a stock, when a wall is slanted, when a rim is rounded or cut only in part or not to one
    /// size, when the walls end at different heights, or when a hole inside goes deeper than the prism.
    pub fn prism_at(&self, body: Id, face: u32, kernel: &dyn crate::feature::Kernel) -> Result<Prism, CoreError> {
        let mi = self.mesh_index(body).ok_or(CoreError::SourceBodyNotBuilt)?;
        let (mesh, faces) = (&self.bodies[mi].mesh, &self.bodies[mi].faces);
        let by_id = |id: u32| faces.iter().find(|f| f.id == id);
        let top = by_id(face).ok_or(CoreError::FaceNotFound)?;
        let n = unit(top.normal);
        // a face standing square to the normal (a wall), lying flat as the face does (a base), or neither (a rim)
        let square = |id: u32| by_id(id).is_some_and(|f| normals(mesh, f).iter().all(|t| (*t).dot(n).abs() <= SQUARE));
        let level = |id: u32| by_id(id).is_some_and(|f| normals(mesh, f).iter().all(|t| 1.0 - (*t).dot(n) <= SQUARE * SQUARE));
        // flat and facing the other way: the far end of the body. Walked over as a rim, it gave the refusal "the rim is
        // rounded" for the side of a block with no rounding on it
        let opposite = |id: u32| by_id(id).is_some_and(|f| normals(mesh, f).iter().all(|t| 1.0 + (*t).dot(n) <= SQUARE * SQUARE));
        if !level(face) {
            return Err(CoreError::PrismFaceNotFlat);
        }
        let o = [top.centroid.x, top.centroid.y, top.centroid.z];
        let pairs = kernel.edge_face_pairs(body);
        let geom: std::collections::HashMap<u32, Vec<[f64; 3]>> = kernel.body_edge_geometry(body).into_iter().map(|(id, poly, _)| (id, poly)).collect();
        let edge_of = |p: &crate::feature::EdgeFaces, of: u32| -> Option<Rim> {
            let poly = geom.get(&p.edge)?;
            let across = if p.faces[0] == of { p.faces[1] } else { p.faces[0] };
            Some(Rim { edge: p.edge, across, from: *poly.first()?, to: *poly.last()? })
        };
        let rims: Vec<Rim> = pairs.iter().filter(|p| p.faces.contains(&face) && p.faces[0] != p.faces[1]).filter_map(|p| edge_of(p, face)).collect();
        let mut chains = loops(rims);
        chains.sort_by(|a, b| area_along(b, n).total_cmp(&area_along(a, n)));
        let mut chains = chains.into_iter();
        let outer = chains.next().ok_or(CoreError::NotAPrism)?;
        let mut first: Vec<u32> = outer.iter().map(|r| r.across).collect();
        first.sort_unstable();
        first.dedup();
        // THE RIM AT THE FACE: walked over from the face across what is neither square nor flat, to the walls
        let at_face = walk(&pairs, Walk { from: &first, kept: &[face] }, |f| !square(f) && !level(f) && !opposite(f));
        if !at_face.over.is_empty() && first.iter().any(|f| square(*f)) {
            return Err(CoreError::PrismRimRounded);
        }
        let walls: Vec<u32> = if at_face.over.is_empty() { first.clone() } else { at_face.stopped.iter().copied().filter(|f| square(*f)).collect() };
        if walls.is_empty() || at_face.stopped.iter().any(|f| !square(*f)) {
            // a walk that met no square wall went down a slanted one
            return Err(CoreError::PrismWallSlanted);
        }
        let rim_at_face = rim_of(mesh, &at_face.over, faces, Along { from: o, n }, 0.0)?;
        // the edges where the walls meet what is above them: the plan of the outline
        let above: Vec<u32> = std::iter::once(face).chain(at_face.over.iter().copied()).collect();
        let upper: Vec<Rim> = pairs.iter().filter(|p| p.faces.iter().any(|f| walls.contains(f)) && p.faces.iter().any(|f| above.contains(f))).filter_map(|p| edge_of(p, p.faces[0])).collect();
        // which way the walls face at those edges: out of the loop (a boss) or into it (a pocket)
        let mut outward = 0.0;
        for r in &upper {
            let p = pairs.iter().find(|p| p.edge == r.edge);
            let Some(w) = p.and_then(|p| p.faces.iter().find(|f| walls.contains(f))).and_then(|&f| by_id(f)) else { continue };
            let mid = [(r.from[0] + r.to[0]) * 0.5, (r.from[1] + r.to[1]) * 0.5, (r.from[2] + r.to[2]) * 0.5];
            let wn = nearest_normal(mesh, w, mid).unwrap_or(w.normal);
            let away = mid.minus(o);
            let away = away.minus([n[0] * away.dot(n), n[1] * away.dot(n), n[2] * away.dot(n)]);
            // weighed by the length of the edge: a long side tells more than a short one round a corner
            outward += wn.dot(away).signum() * r.to.minus(r.from).dot(r.to.minus(r.from)).sqrt().max(NEAR);
        }
        let kind = if outward > 0.0 { PrismKind::Boss } else { PrismKind::Pocket };
        // THE BASE: below the walls, across the rim at the base if there is one, flat faces at one height
        let mut kept: Vec<u32> = above.clone();
        kept.extend(walls.iter().copied());
        let below: Vec<u32> = pairs.iter().filter(|p| p.faces.iter().any(|f| walls.contains(f))).flat_map(|p| p.faces.iter().copied()).filter(|f| !kept.contains(f)).collect();
        let at_base = walk(&pairs, Walk { from: &below, kept: &kept }, |f| !square(f) && !level(f) && !opposite(f));
        if !at_base.over.is_empty() && below.iter().any(|f| level(*f)) {
            return Err(CoreError::PrismRimRounded);
        }
        let base_faces: Vec<u32> = if at_base.over.is_empty() { below.clone() } else { at_base.stopped.clone() };
        // THE FAR END OF THE BODY under the walls: a stock, when the walls end on it alone
        let stock = base_faces.iter().any(|f| opposite(*f));
        if stock && !base_faces.iter().all(|f| opposite(*f)) {
            return Err(CoreError::PrismStockCarries);
        }
        let kind = if stock { PrismKind::Stock } else { kind };
        let mut base = Vec::new();
        let mut height: Option<f64> = None;
        for f in base_faces {
            if !level(f) && !opposite(f) {
                return Err(CoreError::NotAPrism);
            }
            let Some(b) = by_id(f) else { continue };
            let s = ([b.centroid.x, b.centroid.y, b.centroid.z].minus(o)).dot(n);
            if height.is_some_and(|h| (h - s).abs() > NEAR) {
                return Err(CoreError::PrismBaseUneven);
            }
            height = Some(s);
            if !base.contains(&f) {
                base.push(f);
            }
        }
        let s = height.ok_or(CoreError::NotAPrism)?;
        // a boss and a stock end below their face, a pocket above its floor
        if (kind != PrismKind::Pocket) != (s < -NEAR) || s.abs() <= NEAR || (stock && outward <= 0.0) {
            return Err(CoreError::NotAPrism);
        }
        let rim_at_base = rim_of(mesh, &at_base.over, faces, Along { from: o, n }, s)?;
        // loops inside the face: holes through the prism, taken off with it while they end inside it
        let mut inner = Vec::new();
        let mut profile: Vec<u32> = upper.iter().map(|r| r.edge).collect();
        for chain in chains {
            for r in chain {
                let Some(w) = by_id(r.across) else { continue };
                let reach = reach_along(mesh, w, Along { from: o, n }).ok_or(CoreError::FaceNotFound)?;
                let past = reach.low < s.min(0.0) - NEAR || reach.high > s.max(0.0) + NEAR;
                profile.push(r.edge);
                // on a stock a loop that does not run through is something standing on its face - a boss on the top,
                // a pocket in it - and stays; its outline is drawn as a hole of the sketch
                let through = (reach.low - s).abs() <= NEAR && reach.high.abs() <= NEAR;
                if stock && !through {
                    continue;
                }
                if past {
                    return Err(CoreError::PrismHoleDeeper);
                }
                if !inner.contains(&r.across) {
                    inner.push(r.across);
                }
            }
        }
        // on a stock, a loop in its far end going into it - a pocket from that side - the sketch would fill: refused. A
        // boss standing out of the far end stays, as one on the face does
        if stock {
            for &b in &base {
                let rims: Vec<Rim> = pairs.iter().filter(|p| p.faces.contains(&b) && p.faces[0] != p.faces[1]).filter_map(|p| edge_of(p, b)).collect();
                let mut chains = loops(rims);
                chains.sort_by(|x, y| area_along(y, n).total_cmp(&area_along(x, n)));
                for r in chains.iter().skip(1).flatten().filter(|r| !inner.contains(&r.across)) {
                    let Some(w) = by_id(r.across) else { continue };
                    if reach_along(mesh, w, Along { from: o, n }).ok_or(CoreError::FaceNotFound)?.high > s + NEAR {
                        return Err(CoreError::PrismStockCarries);
                    }
                }
            }
        }
        // the outline of the base: the outer loop of its largest face, the loop the prism stands in left out
        let widest = base.iter().filter_map(|&b| by_id(b)).max_by(|a, b| a.area.total_cmp(&b.area)).map(|b| b.id);
        let outline = widest.map_or(Vec::new(), |b| {
            let rims: Vec<Rim> = pairs.iter().filter(|p| p.faces.contains(&b) && p.faces[0] != p.faces[1]).filter_map(|p| edge_of(p, b)).collect();
            let mut chains = loops(rims);
            chains.sort_by(|a, b| area_along(b, n).total_cmp(&area_along(a, n)));
            chains.into_iter().next().map_or(Vec::new(), |c| c.into_iter().map(|r| r.edge).collect())
        });
        let mut rim_faces = at_face.over;
        rim_faces.extend(at_base.over);
        Ok(Prism { face, walls, inner, rim_faces, base, outline, profile, height: s.abs(), kind, rims: Rims { at_face: rim_at_face, at_base: rim_at_base } })
    }
}

/// THE RIM MADE OF `strips`, read from how far they reach along `n` from the plane at `at` from `o`: a rounding when
/// any strip bends, a chamfer when every one is flat; its size the reach. None for no strips; refused when the strips
/// reach unequally.
fn rim_of(mesh: &Mesh, strips: &[u32], faces: &[MeshFace], along: Along, at: f64) -> Result<Option<RimBlend>, CoreError> {
    let Along { from: o, n } = along;
    if strips.is_empty() {
        return Ok(None);
    }
    let mut size: Option<f64> = None;
    let mut bends = false;
    for &id in strips {
        let Some(f) = faces.iter().find(|f| f.id == id) else { continue };
        let reach = reach_along(mesh, f, Along { from: o, n }).ok_or(CoreError::FaceNotFound)?;
        let r = (reach.high - at).abs().max((reach.low - at).abs());
        if size.is_some_and(|s| (s - r).abs() > 1e-3 * s.max(1.0)) {
            return Err(CoreError::PrismRimUneven);
        }
        size = Some(size.map_or(r, |s| s.max(r)));
        let ns = normals(mesh, f);
        bends |= ns.iter().any(|t| ns.first().is_some_and(|f0| 1.0 - (*t).dot(*f0) > SQUARE * SQUARE));
    }
    Ok(size.map(|size| RimBlend { cut: if bends { RimCut::Round } else { RimCut::Chamfer }, size }))
}

/// THE EDGES OF BODY `body` RUNNING ROUND AN OUTLINE AT A HEIGHT: every point of the edge at `height` along the normal of
/// the sketch frame `frame` (a 3 x 4 placement), and within `NEAR_PLAN` of the polyline `outline` in its plane. The edges
/// a rim of a redrawn prism is laid on again: where its new walls meet its new face, or the base.
pub fn edges_round_outline(kernel: &dyn crate::feature::Kernel, body: Id, frame: &[f64; 12], height: f64, outline: &[Point2]) -> Vec<u32> {
    let (x, y, n, o) = ([frame[0], frame[4], frame[8]], [frame[1], frame[5], frame[9]], [frame[2], frame[6], frame[10]], [frame[3], frame[7], frame[11]]);
    let near_line = |p: Point2| -> bool {
        let k = outline.len();
        (0..k).any(|i| {
            let (a, b) = (outline[i], outline[(i + 1) % k]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let l2 = (dx * dx + dy * dy).max(1e-24);
            let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0);
            (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy) < NEAR_PLAN
        })
    };
    kernel
        .body_edge_geometry(body)
        .into_iter()
        .filter(|(_, poly, _)| {
            !poly.is_empty()
                && poly.iter().all(|p| {
                    let q = (*p).minus(o);
                    (q.dot(n) - height).abs() < 1e-3 && near_line(Point2::new(q.dot(x), q.dot(y)))
                })
        })
        .map(|(id, _, _)| id)
        .collect()
}

/// How far from the outline of a sketch an edge laid by it may stand in plan and still be taken for one of it, in mm:
/// the outline of an arc is a polyline, and the true arc stands up to its sagitta off it.
const NEAR_PLAN: f64 = 0.05;

/// The normal of the triangle of face `f` lying nearest to `at`.
fn nearest_normal(mesh: &Mesh, f: &MeshFace, at: [f64; 3]) -> Option<[f64; 3]> {
    f.triangles
        .iter()
        .map(|&t| {
            let [a, b, c] = mesh.triangle(t as usize);
            let mid = [(a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0, (a.z + b.z + c.z) / 3.0];
            let (u, v) = ([b.x - a.x, b.y - a.y, b.z - a.z], [c.x - a.x, c.y - a.y, c.z - a.z]);
            let nn = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            Near { far: mid.minus(at).dot(mid.minus(at)), normal: nn }
        })
        .filter(|x| x.normal.dot(x.normal) > 1e-24)
        .min_by(|a, b| a.far.total_cmp(&b.far))
        .map(|x| unit(x.normal))
}

/// A triangle of a face: how far it lies from a point (squared), and its normal.
struct Near {
    far: f64,
    normal: [f64; 3],
}
