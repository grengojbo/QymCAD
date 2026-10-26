//! THE JOINTS OF A CUT TO FIT, laid where they hold: on the face every cut left on the piece behind it, spots that keep a
//! socket and the edge of a wall clear of the face's rim - two round pegs as far apart as the face holds them, so the
//! pieces cannot turn on one, or one hex peg, which cannot turn - each pair of pieces across a cut held by one node. A
//! face too small for a socket is left without, and told.

use crate::print::checks::Bed;
use crate::print::split::{split_in_batch, Cut};
use crate::{DocEngine, DocError};
use qymcad_core::feature::{PegShape, PegSpec, PlaneAt};
use qymcad_core::geom::Mesh;
use qymcad_core::model::{Id, Project};

/// What joints a cut is asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointAsk {
    pub spec: PegSpec,
    /// How much of the face is left between a socket and the face's rim, mm: a wall that prints.
    pub edge: f64,
}

/// The joints laid across one face of a cut: the node, and where its pegs stand.
#[derive(Clone, Debug, PartialEq)]
pub struct Laid {
    pub node: Id,
    pub spots: Vec<[f64; 3]>,
}

/// What a cut to fit with joints left.
#[derive(Clone, Debug, PartialEq)]
pub struct Joined {
    /// The pieces to print, the pins of dowels among them.
    pub pieces: Vec<Id>,
    pub cuts: Vec<Cut>,
    pub joints: Vec<Laid>,
    /// Faces of a cut too small for a socket, or with no piece across: left without joints.
    pub left_without: usize,
}

/// A face lies on a cut within this, mm.
const ON_CUT: f64 = 1e-6;

/// THE PIECES OF `body` CUT TO FIT `bed` less `margin` AND JOINED ACROSS EVERY CUT as `ask` asks, as one step.
pub fn cut_and_join(doc: &mut DocEngine, body: Id, bed: &Bed, margin: f64, ask: &JointAsk) -> Result<Joined, DocError> {
    if crate::print::split::cuts_of(doc, body, bed, margin)?.is_empty() {
        return Ok(Joined { pieces: vec![body], cuts: Vec::new(), joints: Vec::new(), left_without: 0 });
    }
    doc.begin_batch("f-operation");
    match cut_and_join_in_batch(doc, body, bed, margin, ask) {
        Ok(j) => {
            doc.commit_batch();
            Ok(j)
        }
        Err(e) => {
            doc.abort_batch();
            Err(e)
        }
    }
}

fn cut_and_join_in_batch(doc: &mut DocEngine, body: Id, bed: &Bed, margin: f64, ask: &JointAsk) -> Result<Joined, DocError> {
    let split = split_in_batch(doc, body, bed, margin)?;
    let mut pieces = split.pieces.clone();
    let mut joints = Vec::new();
    let mut left_without = 0;
    let socket = ask.spec.diameter / 2.0 + ask.spec.clearance;
    let count = match ask.spec.shape {
        PegShape::Round => 2,
        PegShape::Hex => 1,
    };
    for cut in &split.cuts {
        let mut n = [0.0; 3];
        n[cut.axis] = 1.0;
        // every face the cut left on a piece behind it, with the spots it holds - read before any joint changes a piece
        let mut faces: Vec<FaceOnCut> = Vec::new();
        for &piece in &pieces {
            for tris in cut_faces(doc.project(), piece, cut, 1.0) {
                faces.push(FaceOnCut { piece, spots: spots_on(&tris, cut, socket + ask.edge, count) });
            }
        }
        for f in faces {
            let Some(first) = f.spots.first().copied() else {
                left_without += 1;
                continue;
            };
            let behind = pieces.iter().copied().find(|&p| p == f.piece);
            let front = pieces.iter().copied().find(|&p| p != f.piece && cut_faces(doc.project(), p, cut, -1.0).iter().any(|t| covers(t, cut, first)));
            let (Some(a), Some(b)) = (behind, front) else {
                left_without += 1;
                continue;
            };
            let at = PlaneAt { origin: first, normal: n };
            let spots = f.spots.clone();
            let spec = ask.spec;
            let made = doc.edit("f-operation", |p| Ok(p.add_joined([a, b], at, spec, spots)))?;
            pieces.retain(|&p| p != a && p != b);
            pieces.extend(made.iter().copied());
            joints.push(Laid { node: made[0], spots: f.spots });
        }
    }
    Ok(Joined { pieces, cuts: split.cuts, joints, left_without })
}

/// A face a cut left on a piece behind it, and the spots it holds.
struct FaceOnCut {
    piece: Id,
    spots: Vec<[f64; 3]>,
}

/// The triangles of every face of `piece` lying on `cut` and looking `side` along its axis (+1 the way of the axis, the
/// face of a piece behind the cut), each face its own list, in the frame of the part.
fn cut_faces(project: &Project, piece: Id, cut: &Cut, side: f64) -> Vec<Vec<[[f64; 3]; 3]>> {
    let Some(mi) = project.mesh_index(piece) else { return Vec::new() };
    let mesh: &Mesh = &project.bodies[mi].mesh;
    let faces = project.regen_faces.get(&piece).map(Vec::as_slice).unwrap_or(project.bodies[mi].faces.as_slice());
    faces
        .iter()
        .filter(|f| f.normal[cut.axis] * side > 0.999 && f.triangles.iter().all(|&t| mesh.triangle(t as usize).iter().all(|p| ([p.x, p.y, p.z][cut.axis] - cut.at).abs() < ON_CUT)))
        .map(|f| f.triangles.iter().map(|&t| mesh.triangle(t as usize).map(|p| [p.x, p.y, p.z])).collect())
        .collect()
}

/// The two axes across a cut, in their order.
fn across(cut: &Cut) -> [usize; 2] {
    match cut.axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}

/// Whether the face of triangles `tris` holds the point `at` of the cut.
fn covers(tris: &[[[f64; 3]; 3]], cut: &Cut, at: [f64; 3]) -> bool {
    let [u, v] = across(cut);
    tris.iter().any(|t| inside([[t[0][u], t[0][v]], [t[1][u], t[1][v]], [t[2][u], t[2][v]]], [at[u], at[v]]))
}

/// Whether the point `p` lies within the triangle `t`, on the plane.
fn inside(t: [[f64; 2]; 3], p: [f64; 2]) -> bool {
    let side = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let (d0, d1, d2) = (side(t[0], t[1]), side(t[1], t[2]), side(t[2], t[0]));
    let neg = d0 < -1e-12 || d1 < -1e-12 || d2 < -1e-12;
    let pos = d0 > 1e-12 || d1 > 1e-12 || d2 > 1e-12;
    !(neg && pos)
}

/// The points of the plane a grid is laid over a face with, along each side at most: 4 900 points tried.
const GRID: usize = 70;

/// UP TO `count` SPOTS ON THE FACE `tris` of `cut`, each `clear` mm at least from the face's rim: one the nearest the
/// middle of the face, two the farthest apart of those that hold. None where no point holds.
fn spots_on(tris: &[[[f64; 3]; 3]], cut: &Cut, clear: f64, count: usize) -> Vec<[f64; 3]> {
    let [u, v] = across(cut);
    let flat: Vec<[[f64; 2]; 3]> = tris.iter().map(|t| [[t[0][u], t[0][v]], [t[1][u], t[1][v]], [t[2][u], t[2][v]]]).collect();
    // the rim: the sides of the triangles no second triangle shares
    let key = |a: [f64; 2]| [(a[0] * 1e6).round() as i64, (a[1] * 1e6).round() as i64];
    let mut sides: std::collections::HashMap<[i64; 4], usize> = std::collections::HashMap::new();
    for t in &flat {
        for k in 0..3 {
            let (a, b) = (key(t[k]), key(t[(k + 1) % 3]));
            let s = if a < b { [a[0], a[1], b[0], b[1]] } else { [b[0], b[1], a[0], a[1]] };
            *sides.entry(s).or_insert(0) += 1;
        }
    }
    let rim: Vec<[[f64; 2]; 2]> = sides.into_iter().filter(|(_, n)| *n == 1).map(|(s, _)| [[s[0] as f64 / 1e6, s[1] as f64 / 1e6], [s[2] as f64 / 1e6, s[3] as f64 / 1e6]]).collect();
    let off_rim = |p: [f64; 2]| rim.iter().map(|s| to_segment(p, *s)).fold(f64::INFINITY, f64::min);
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for t in &flat {
        for q in t {
            for k in 0..2 {
                lo[k] = lo[k].min(q[k]);
                hi[k] = hi[k].max(q[k]);
            }
        }
    }
    if !(lo[0].is_finite() && hi[0] > lo[0] && hi[1] > lo[1]) {
        return Vec::new();
    }
    let mut held: Vec<[f64; 2]> = Vec::new();
    for i in 0..=GRID {
        for j in 0..=GRID {
            let p = [lo[0] + (hi[0] - lo[0]) * i as f64 / GRID as f64, lo[1] + (hi[1] - lo[1]) * j as f64 / GRID as f64];
            if flat.iter().any(|t| inside(*t, p)) && off_rim(p) >= clear - 1e-9 {
                held.push(p);
            }
        }
    }
    let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    let far = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2);
    let chosen: Vec<[f64; 2]> = match (count, held.len()) {
        (_, 0) => Vec::new(),
        (1, _) | (_, 1) => held.iter().copied().min_by(|a, b| far(*a, mid).total_cmp(&far(*b, mid))).into_iter().collect(),
        _ => {
            // the farthest from the middle, then the farthest from it
            let first = held.iter().copied().max_by(|a, b| far(*a, mid).total_cmp(&far(*b, mid))).unwrap_or(mid);
            let second = held.iter().copied().max_by(|a, b| far(*a, first).total_cmp(&far(*b, first))).unwrap_or(mid);
            // two sockets that would meet are one place: one peg, the nearest the middle
            if far(first, second).sqrt() < 2.0 * clear {
                held.iter().copied().min_by(|a, b| far(*a, mid).total_cmp(&far(*b, mid))).into_iter().collect()
            } else {
                vec![first, second]
            }
        }
    };
    chosen
        .into_iter()
        .map(|p| {
            let mut s = [0.0; 3];
            s[cut.axis] = cut.at;
            s[u] = p[0];
            s[v] = p[1];
            s
        })
        .collect()
}

/// How far the point `p` is from the segment `s`.
fn to_segment(p: [f64; 2], s: [[f64; 2]; 2]) -> f64 {
    let d = [s[1][0] - s[0][0], s[1][1] - s[0][1]];
    let l2 = d[0] * d[0] + d[1] * d[1];
    let t = if l2 > 0.0 { (((p[0] - s[0][0]) * d[0] + (p[1] - s[0][1]) * d[1]) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let q = [s[0][0] + d[0] * t, s[0][1] + d[1] * t];
    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt()
}
