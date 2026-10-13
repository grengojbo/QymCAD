//! SPLITTING A PREPARED MESH INTO REGIONS, each lying on one surface: a plane, a cylinder, a cone, a sphere or a
//! torus.
//!
//! By the angle between triangles alone it cannot be done. Measured on a bar with every edge rounded (26 faces),
//! the angle across the border of two faces was 0.12 - 6.5 deg while the angle inside a rounding reached 14.3 deg:
//! the faces meet tangent, and any threshold that keeps a rounding whole glues the whole bar into one region. So a
//! region is grown together with its surface. A seed is given a plane, cylinders, cones and spheres in turn; each
//! grows over the neighbours whose corners lie on it; the one that covers the most area is kept. A rounding's first
//! strip lies 0.017 mm off the plane it leaves, far outside the tolerance. Then neighbours whose union lies on one
//! surface are joined - that is how a torus comes together from its bands - and what is left lying on a larger
//! neighbour's surface joins it.

use crate::{Prepared, NO_NEIGHBOUR};
use nalgebra::{DMatrix, DVector, Matrix3, Matrix4, SymmetricEigen, Vector3, Vector4};
use std::collections::{HashMap, HashSet, VecDeque};

/// A surface a region lies on.
#[derive(Clone, Debug, PartialEq)]
pub enum Surface {
    Plane {
        point: [f64; 3],
        normal: [f64; 3],
    },
    /// `point` lies on the axis, `axis` is a unit vector.
    Cylinder {
        point: [f64; 3],
        axis: [f64; 3],
        radius: f64,
    },
    Sphere {
        center: [f64; 3],
        radius: f64,
    },
    /// `apex` is the tip, `axis` a unit vector from the tip into the cone, `half_angle` in radians.
    Cone {
        apex: [f64; 3],
        axis: [f64; 3],
        half_angle: f64,
    },
    /// `axis` is a unit vector square to the torus's middle plane through `center`; `major` is the radius of the
    /// tube's centre line, `minor` the tube's.
    Torus {
        center: [f64; 3],
        axis: [f64; 3],
        major: f64,
        minor: f64,
    },
    /// A SMOOTH WALL FITTED BY ONE B-SPLINE SURFACE through the corners of the whole region - a free form known to the
    /// last pole, where `Free` leaves it to the kernel to fill the border.
    Spline(Box<crate::bspline::BSpline>),
    /// No surface of the kinds above: a smooth free form, its triangles all facing within a right angle of `normal`, the
    /// mean of them, which points out. What it is exactly the kernel finds, filling the face's own border.
    Free {
        normal: [f64; 3],
    },
    /// A straight profile screwed about an axis: the flank of a thread, an auger's flight. `point` lies on the axis;
    /// `axis` and `reference` are unit vectors square to each other. A point at distance r from the axis, height z along
    /// it from `point` and angle t from `reference`, turning right-handed about `axis`, lies on it where
    /// `radial * r + axial * (z - rise * t) = offset` with t taken some whole number of turns on: `rise` is the height
    /// gained per radian, negative for a left-hand screw, and (`radial`, `axial`) the unit normal of the profile's line
    /// in the cut through the axis, `axial` positive. Its own normal points the way (`radial`, `axial`) does.
    Helix {
        point: [f64; 3],
        axis: [f64; 3],
        reference: [f64; 3],
        rise: f64,
        radial: f64,
        axial: f64,
        offset: f64,
    },
    /// A circle screwed about an axis: a rounded edge of an auger's flight, the root of a thread drawn round. As `Helix`,
    /// but a point lies on it where, taken the whole turns that bring z - rise * t nearest `height`, its distance from the
    /// axis and z - rise * t lie `round` from (`middle`, `height`). Its own normal points away from the circle's middle.
    RoundHelix {
        point: [f64; 3],
        axis: [f64; 3],
        reference: [f64; 3],
        rise: f64,
        middle: f64,
        height: f64,
        round: f64,
    },
    /// A round wire wound about an axis: a spring's coil. `point` lies on the axis; `axis` and `reference` are unit
    /// vectors square to each other. The wire's middle runs `radius` from the axis at angle t from `reference`, turning
    /// right-handed about `axis`, `lift + rise * t` along it; the surface is every point `wire` from that line. Its own
    /// normal points away from the line.
    Coil {
        point: [f64; 3],
        axis: [f64; 3],
        reference: [f64; 3],
        rise: f64,
        radius: f64,
        lift: f64,
        wire: f64,
    },
}

impl Surface {
    /// How far `v` lies from the surface, in mm.
    pub fn distance(&self, v: [f64; 3]) -> f64 {
        distance(self, &Vector3::from(v))
    }

    /// How far `v` lies off a helix, a round helix or a coil, the side its own normal points to positive; zero on any other
    /// surface.
    pub(crate) fn signed_off(&self, v: &Vector3<f64>) -> f64 {
        match self {
            Surface::Coil { .. } => coil_at(self, v).map_or(0.0, |c| c.off),
            _ => helix_at(self, v).map_or(0.0, |h| h.off),
        }
    }

    /// Where `v` stands on a helix: its angle round the axis with the whole turns that put it on the helix, and its
    /// distance from the axis - on a round helix its angle round the profile's circle. `None` on any other surface.
    pub fn turn_at(&self, v: [f64; 3]) -> Option<(f64, f64)> {
        match self {
            Surface::Coil { .. } => coil_at(self, &Vector3::from(v)).map(|c| (c.turn, 0.0)),
            _ => helix_at(self, &Vector3::from(v)).map(|h| (h.turn, h.across)),
        }
    }

    /// The unit normal of the surface at the point nearest `v`, or `None` where it has none there.
    pub fn normal_at(&self, v: [f64; 3]) -> Option<[f64; 3]> {
        normal_at(self, &Vector3::from(v)).map(Into::into)
    }
}

/// Where a point stands about a helix.
struct OnHelix {
    /// The angle round the axis, with the whole turns that bring the point onto the helix.
    turn: f64,
    /// Its distance from the axis, or on a round helix its angle round the profile's circle from the way out.
    across: f64,
    /// The helix's own normal there, not of unit length.
    normal: Vector3<f64>,
    /// How far off the helix the point is, the side the normal points positive: the miss within half a turn, divided by
    /// how steeply it grows.
    off: f64,
}

fn helix_at(s: &Surface, v: &Vector3<f64>) -> Option<OnHelix> {
    if let Surface::RoundHelix { point, axis, reference, rise, middle, height, round } = *s {
        let (a, x) = (Vector3::from(axis), Vector3::from(reference));
        let d = v - Vector3::from(point);
        let z = d.dot(&a);
        let e = d - a * z;
        let far = e.norm();
        let out = e.try_normalize(1e-12)?;
        let seen = out.dot(&a.cross(&x)).atan2(out.dot(&x));
        let pitch = 2.0 * std::f64::consts::PI * rise;
        let lifted = z - rise * seen;
        let turns = if pitch.abs() > 1e-300 { ((lifted - height) / pitch).round() } else { 0.0 };
        let (dr, dz) = (far - middle, lifted - pitch * turns - height);
        let apart = dr.hypot(dz).max(1e-300);
        let normal = out * (dr / apart) + (a - a.cross(&out) * (rise / far)) * (dz / apart);
        return Some(OnHelix { turn: seen + 2.0 * std::f64::consts::PI * turns, across: dz.atan2(dr), off: (apart - round) / normal.norm(), normal });
    }
    let Surface::Helix { point, axis, reference, rise, radial, axial, offset } = *s else { return None };
    let (a, x) = (Vector3::from(axis), Vector3::from(reference));
    let d = v - Vector3::from(point);
    let z = d.dot(&a);
    let e = d - a * z;
    let far = e.norm();
    let out = e.try_normalize(1e-12)?;
    let round = a.cross(&out);
    let seen = out.dot(&a.cross(&x)).atan2(out.dot(&x));
    let period = 2.0 * std::f64::consts::PI * axial * rise;
    let raw = radial * far + axial * (z - rise * seen) - offset;
    let turns = if period.abs() > 1e-300 { (raw / period).round() } else { 0.0 };
    let normal = out * radial + (a - round * (rise / far)) * axial;
    Some(OnHelix { turn: seen + 2.0 * std::f64::consts::PI * turns, across: far, off: (raw - period * turns) / normal.norm(), normal })
}

/// Where a point stands about a coil.
struct OnCoil {
    /// The angle round the axis of the nearest point of the wire's middle, with its whole turns.
    turn: f64,
    /// The way from that point out to `v`, not of unit length.
    out: Vector3<f64>,
    /// How far off the coil `v` is, outside positive.
    off: f64,
}

/// Where `v` stands about a coil: the nearest point of the wire's middle, found from the angle `v` stands at - with the
/// whole turns its height gives - by Newton's steps on (v - H(t)) . H'(t) = 0.
fn coil_at(s: &Surface, v: &Vector3<f64>) -> Option<OnCoil> {
    let Surface::Coil { point, axis, reference, rise, radius, lift, wire } = *s else { return None };
    let (o, a, x) = (Vector3::from(point), Vector3::from(axis), Vector3::from(reference));
    let y = a.cross(&x);
    let d = v - o;
    let seen = d.dot(&y).atan2(d.dot(&x));
    let tau = 2.0 * std::f64::consts::PI;
    let mut turn = if rise.abs() > 1e-12 { seen + tau * (((d.dot(&a) - lift) / rise - seen) / tau).round() } else { seen };
    for _ in 0..6 {
        let (sin, cos) = turn.sin_cos();
        let at = o + (x * cos + y * sin) * radius + a * (lift + rise * turn);
        let ahead = (y * cos - x * sin) * radius + a * rise;
        let bend = -(x * cos + y * sin) * radius;
        let slope = -ahead.norm_squared() + (v - at).dot(&bend);
        if slope.abs() < 1e-300 {
            break;
        }
        turn -= (v - at).dot(&ahead) / slope;
    }
    let (sin, cos) = turn.sin_cos();
    let out = v - (o + (x * cos + y * sin) * radius + a * (lift + rise * turn));
    Some(OnCoil { turn, off: out.norm() - wire, out })
}

/// The triangles of one region, and the surface they lie on; `None` where no surface was found.
#[derive(Clone, Debug)]
pub struct Region {
    pub tris: Vec<u32>,
    pub surface: Option<Surface>,
}

/// How strictly a region is grown.
#[derive(Clone, Copy, Debug)]
pub struct Tolerance {
    /// How far a corner may lie from the surface, in mm.
    pub distance: f64,
    /// How far a triangle's normal may turn from the surface's, in degrees.
    pub angle_deg: f64,
    /// An edge sharper than this, in degrees, is never crossed.
    pub sharp_deg: f64,
}

impl Tolerance {
    /// A hundred-thousandth of the diagonal: 0.00046 mm on a 46 mm bar. A mesh from CAD puts its corners on the
    /// surface to 1e-10, and a text STL rounds them to six digits - 2e-4 mm on that bar - so this is room enough
    /// for both. It is not wider because tangent neighbours come close: measured on the rounded bar, the corners of
    /// a corner sphere's triangles next to a rounding lie 0.0025 mm off its cylinder, and a ten-thousandth of the
    /// diagonal let them in. The normal may turn 10 deg - a guard against a folded triangle, not what tells faces
    /// apart: the distance does that. At 2 deg the big skewed triangles along a cone's base, lying on the cone to the
    /// last digit, stood apart as 32 regions of one or two.
    pub fn for_mesh(p: &Prepared) -> Tolerance {
        let d = p.mesh.bounds().map(|b| ((b.max.x - b.min.x).powi(2) + (b.max.y - b.min.y).powi(2) + (b.max.z - b.min.z).powi(2)).sqrt()).unwrap_or(1.0);
        Tolerance { distance: (d * 1e-5).max(1e-7), angle_deg: 10.0, sharp_deg: 30.0 }
    }
}

const NONE: u32 = u32::MAX;

/// How much more area a cylinder must cover than a plane, or a sphere than either, to be taken instead.
const SIMPLER_WINS: f64 = 1.5;

/// The share of a region that a refit may leave behind: the triangles that no longer lie on the better surface go
/// back to be taken by their own.
const MAY_LEAVE: usize = 10;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Plane,
    Cylinder,
    Cone,
    Sphere,
    Torus,
}

/// Split `p` into regions. Every triangle ends in exactly one.
pub fn regions(p: &Prepared, tol: &Tolerance) -> Vec<Region> {
    regions_until(p, tol, &|| false).unwrap_or_default()
}

/// The same, asking `stop` before every seed: `None` once it says yes. Growing the regions is most of a recognition
/// (11 s of 14 s on a 160k-triangle ball in a debug build), and work nobody waits for any more is dropped at once.
pub fn regions_until(p: &Prepared, tol: &Tolerance, stop: &dyn Fn() -> bool) -> Option<Vec<Region>> {
    let n = p.mesh.tris.len();
    let mut sliver = vec![false; n];
    for &s in &p.slivers {
        sliver[s as usize] = true;
    }
    let g = Grower { p, tol, cos_sharp: tol.sharp_deg.to_radians().cos(), cos_angle: tol.angle_deg.to_radians().cos(), sliver: &sliver };
    let area: Vec<f64> = (0..n).map(|t| g.area(t)).collect();
    // THE LARGEST TRIANGLES SEED FIRST: a plane face is often a couple of large triangles, and a surface is guessed
    // best where it is sampled widest
    let mut order: Vec<usize> = (0..n).filter(|&t| !sliver[t]).collect();
    order.sort_by(|&a, &b| area[b].total_cmp(&area[a]).then(a.cmp(&b)));
    let mut owner = vec![NONE; n];
    let mut out: Vec<Region> = Vec::new();
    for seed in order {
        if owner[seed] != NONE {
            continue;
        }
        if stop() {
            return None;
        }
        let r = out.len() as u32;
        match g.widest_at(seed, &owner, &area) {
            Some(Grown { tris, surface: s, .. }) => {
                for &t in &tris {
                    owner[t as usize] = r;
                }
                out.push(Region { tris, surface: Some(s) });
            }
            None => {
                owner[seed] = r;
                out.push(Region { tris: vec![seed as u32], surface: None });
            }
        }
    }
    if stop() {
        return None;
    }
    merge(&g, &mut out, &mut owner);
    absorb(&g, &mut out, &mut owner);
    coaxial(&g, &mut out, &mut owner);
    // THE PIECES TURNED ABOUT ONE AXIS ARE JOINED BY A FIT OF THEIR UNION, not by the axis they were turned about: that
    // axis is some one region's, and measured under a sphere set 0.1 mm off a cylinder's axis it stood 0.0009 mm off
    // the true one, so the four pieces of the wall took radii from 9.9993 to 10.0007 - further apart than the
    // tolerance of 0.0005 - and none of them lay on another's surface.
    merge(&g, &mut out, &mut owner);
    absorb(&g, &mut out, &mut owner);
    helical(&g, &mut out, &mut owner);
    absorb(&g, &mut out, &mut owner);
    // A SLIVER HAS NO NORMAL TO AGREE WITH: it joins a neighbouring region whose surface its corners lie on, or
    // stands alone, unrecognised
    for &s in &p.slivers {
        let s = s as usize;
        let home = p.neighbours[s].iter().filter(|&&u| u != NO_NEIGHBOUR).map(|&u| owner[u as usize]).find(|&r| r != NONE && out[r as usize].surface.as_ref().is_some_and(|f| g.corners_on(s, f)));
        match home {
            Some(r) => {
                owner[s] = r;
                out[r as usize].tris.push(s as u32);
            }
            None => {
                owner[s] = out.len() as u32;
                out.push(Region { tris: vec![s as u32], surface: None });
            }
        }
    }
    coiled(&g, &mut out, &mut owner);
    free_form(&g, &mut out, &mut owner);
    unwind(&g, &mut out, &mut owner);
    if stop() {
        return None;
    }
    free_walls(&g, &mut out, &mut owner);
    out.retain(|r| !r.tris.is_empty());
    Some(apart(&g, out))
}

/// THE REGION GROWN FROM ONE TRIANGLE, as `regions` grows each of its own from a seed: the surfaces of every kind tried
/// at `seed`, each grown over the neighbours lying on it, and the one covering most kept. Where a person points at one
/// place of a body - a hole, a rounding - only the surface there is asked, and the rest of the mesh is not split. `None`
/// where no surface holds the seed.
pub fn region_at(p: &Prepared, tol: &Tolerance, seed: usize) -> Option<Region> {
    let n = p.mesh.tris.len();
    if seed >= n {
        return None;
    }
    let mut sliver = vec![false; n];
    for &s in &p.slivers {
        sliver[s as usize] = true;
    }
    let g = Grower { p, tol, cos_sharp: tol.sharp_deg.to_radians().cos(), cos_angle: tol.angle_deg.to_radians().cos(), sliver: &sliver };
    let area: Vec<f64> = (0..n).map(|t| g.area(t)).collect();
    let owner = vec![NONE; n];
    g.widest_at(seed, &owner, &area).map(|w| Region { tris: w.tris, surface: Some(w.surface) })
}

/// A region grown from a seed: its triangles, the surface they lie on, and the area they cover.
struct Grown {
    area: f64,
    tris: Vec<u32>,
    surface: Surface,
}

/// EVERY REGION IN ONE PIECE: a region whose triangles fall apart - the pieces of one cylinder joined across a slot, the
/// parts of a wall on either side of a hole - becomes one region per piece, each on the same surface. A face has one
/// outer loop, and one built on the loops of several pieces was refused: measured on a simplified handle, 10 walls and
/// 3 cylinders of it, up to 10 pieces side by side, and the body not sound until mended.
///
/// A CRUMB FALLEN OFF - under `CRUMB` triangles, not its region's largest piece or closed round by one piece - joins
/// the piece beside it that shares most of its sides, where that one's surface holds its corners: standing alone, 84 such bits of
/// cylinders of the handle made no face and went back as 468 triangles of mesh.
fn apart(g: &Grower, out: Vec<Region>) -> Vec<Region> {
    let p = g.p;
    let mut region_of = vec![u32::MAX; p.mesh.tris.len()];
    for (r, region) in out.iter().enumerate() {
        for &t in &region.tris {
            region_of[t as usize] = r as u32;
        }
    }
    let mut seen = vec![false; p.mesh.tris.len()];
    let mut pieces: Vec<Region> = Vec::with_capacity(out.len());
    let mut largest: Vec<bool> = Vec::with_capacity(out.len());
    for (r, region) in out.into_iter().enumerate() {
        let first = pieces.len();
        for &s in &region.tris {
            if seen[s as usize] {
                continue;
            }
            seen[s as usize] = true;
            let (mut piece, mut queue) = (vec![s], VecDeque::from([s]));
            while let Some(t) = queue.pop_front() {
                for &u in &p.neighbours[t as usize] {
                    if u != NO_NEIGHBOUR && region_of[u as usize] == r as u32 && !seen[u as usize] {
                        seen[u as usize] = true;
                        piece.push(u);
                        queue.push_back(u);
                    }
                }
            }
            pieces.push(Region { tris: piece, surface: region.surface.clone() });
            largest.push(false);
        }
        if let Some(big) = (first..pieces.len()).max_by_key(|&k| pieces[k].tris.len()) {
            largest[big] = true;
        }
    }
    // the crumbs to their neighbours
    let mut piece_of = vec![u32::MAX; p.mesh.tris.len()];
    for (k, piece) in pieces.iter().enumerate() {
        for &t in &piece.tris {
            piece_of[t as usize] = k as u32;
        }
    }
    for k in 0..pieces.len() {
        if pieces[k].tris.len() >= CRUMB {
            continue;
        }
        let mut sides: HashMap<u32, usize> = HashMap::new();
        for &t in &pieces[k].tris {
            for &u in &p.neighbours[t as usize] {
                if u != NO_NEIGHBOUR && piece_of[u as usize] != k as u32 && piece_of[u as usize] != u32::MAX {
                    *sides.entry(piece_of[u as usize]).or_default() += 1;
                }
            }
        }
        // a whole region of a few triangles stays itself, unless one piece closes round it: a hole of a sliver in a wall
        // is a loop of no area, and the wall's face was refused - two walls of 240 triangles of the handle went back
        // to the mesh
        if largest[k] && sides.len() != 1 {
            continue;
        }
        let mut by: Vec<(u32, usize)> = sides.into_iter().collect();
        by.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        // on a fitted wall, within its flattening and near it: past the square it was fitted over the surface runs off
        // wild - a crumb joined to a wall beyond it put a corner 3.9 of the wall's widths away, where the surface stood
        // 1 155 mm off, and the wall made no face
        let on_wall = |t: u32, w: &crate::bspline::BSpline| {
            (0..3).all(|c| {
                let q = g.corner(t as usize, c);
                let (u, v) = w.uv_of([q.x, q.y, q.z]);
                let at = w.at(u, v);
                (-0.02..=1.02).contains(&u) && (-0.02..=1.02).contains(&v) && ((at[0] - q.x).powi(2) + (at[1] - q.y).powi(2) + (at[2] - q.z).powi(2)).sqrt() <= 10.0 * g.tol.distance
            })
        };
        let home = by.into_iter().map(|(h, _)| h as usize).find(|&h| match &pieces[h].surface {
            Some(Surface::Spline(w)) => pieces[k].tris.iter().all(|&t| on_wall(t, w)),
            Some(s) => pieces[k].tris.iter().all(|&t| g.corners_on(t as usize, s)),
            None => false,
        });
        if let Some(h) = home {
            let moved = std::mem::take(&mut pieces[k].tris);
            for &t in &moved {
                piece_of[t as usize] = h as u32;
            }
            pieces[h].tris.extend(moved);
        }
    }
    pieces.retain(|r| !r.tris.is_empty());
    pieces
}

/// Under how many triangles a piece fallen off its region is a crumb.
const CRUMB: usize = 6;

/// How few triangles make a wall worth its own surface.
const WALL_FROM: usize = 30;

/// A SMOOTH WALL IS ONE SURFACE, NOT A MOSAIC. Where no plane, cylinder, cone, sphere or torus lies on a smooth wall, the
/// seeds grow each of them over a strip of it, as far as it stays within the tolerance: measured on a 178 032-triangle
/// mesh of a smooth handle, 1 887 strips of cylinders and 1 061 regions of none, the body built of 54 825 faces in 530 s.
/// So the tiles of such a wall - curved regions, small planes and triangles no surface took - joined across edges that
/// are not sharp, are cut into pieces that face one way, and on each piece one B-spline surface is fitted to its corners;
/// a piece it holds within the tolerance, replacing two tiles or more, becomes one region on it. A piece it does not hold
/// is halved until each part holds.
fn free_walls(g: &Grower, out: &mut Vec<Region>, owner: &mut [u32]) {
    let tile = |r: &Region| match &r.surface {
        None => true,
        Some(Surface::Plane { .. }) => r.tris.len() <= TILE_UP_TO,
        Some(Surface::Cylinder { .. } | Surface::Cone { .. } | Surface::Sphere { .. } | Surface::Torus { .. }) => true,
        Some(_) => false,
    };
    let n = g.p.mesh.tris.len();
    let in_wall: Vec<bool> = (0..n).map(|t| owner[t] != NONE && tile(&out[owner[t] as usize]) && !g.sliver[t]).collect();
    let mut taken = vec![false; n];
    let mut order: Vec<usize> = (0..n).filter(|&t| in_wall[t]).collect();
    order.sort_by(|&a, &b| g.area(b).total_cmp(&g.area(a)).then(a.cmp(&b)));
    let sag = crate::prep::chord_deflection(g.p, g.tol.sharp_deg);
    let cos_turn = WALL_TURN_DEG.to_radians().cos();
    for seed in order {
        if taken[seed] {
            continue;
        }
        // PIECES OF A WALL THAT FACE ONE WAY, grown from its biggest triangles while each faces within `WALL_TURN_DEG` of
        // the mean so far. Taken whole instead, a smooth handle's walls fitted as they stood, and the faces of the
        // cylinders beside them came out of the mending with copies of their edges that nothing else shared: 33 sides
        // left for the sewing, and the body refused.
        let piece = grow(g, seed, &in_wall, &mut taken, cos_turn);
        // A PIECE NO ONE SURFACE HOLDS IS HALVED, and each half fitted again, until every part holds or is too small to be
        // worth a surface of its own.
        let mut work = vec![piece];
        while let Some(piece) = work.pop() {
            if into_wall(g, out, owner, &piece, sag) == Some(false) && piece.len() >= 2 * WALL_FROM {
                for half in halves(g, &piece) {
                    work.extend(connected(g, &half));
                }
            }
        }
    }
}

/// How far a piece of a wall may turn from where it faces on the whole: a fitted surface lies the wall flat on the plane
/// square to its mean normal, and the fit holds a turn of 75 deg; the piece is cut short of it.
const WALL_TURN_DEG: f64 = 60.0;

/// The triangles joined to `seed` over sides that are not sharp, among those `within` and not `taken` yet (all of them
/// taken on the way), while each faces within `cos_turn` of the mean so far.
fn grow(g: &Grower, seed: usize, within: &[bool], taken: &mut [bool], cos_turn: f64) -> Vec<usize> {
    let mut piece = vec![seed];
    taken[seed] = true;
    let mut sum = g.normal(seed) * g.area(seed);
    let mut queue = VecDeque::from([seed]);
    while let Some(t) = queue.pop_front() {
        for &u in &g.p.neighbours[t] {
            let u = u as usize;
            if u as u32 == NO_NEIGHBOUR || taken[u] || !within[u] || g.sharp(t, u) {
                continue;
            }
            let mean = sum.try_normalize(1e-300).unwrap_or_else(|| g.normal(seed));
            if g.normal(u).dot(&mean) < cos_turn {
                continue;
            }
            taken[u] = true;
            sum += g.normal(u) * g.area(u);
            piece.push(u);
            queue.push_back(u);
        }
    }
    piece
}

/// `piece` made one region on one fitted wall, where it is a mosaic of two tiles or more and one surface holds it; the
/// tiles give its triangles up. `None` where it is no mosaic - its tiles are the part's faces and it is left alone, its
/// parts too: halved, parts of such pieces passed for mosaics, and on a smooth handle 126 of its 264 cylinders went into
/// walls and the body was refused - and otherwise whether one surface held it.
fn into_wall(g: &Grower, out: &mut Vec<Region>, owner: &mut [u32], piece: &[usize], sag: f64) -> Option<bool> {
    let tiles: HashSet<u32> = piece.iter().map(|&t| owner[t]).collect();
    if piece.len() < WALL_FROM || tiles.len() < 2 || !(bends_many_ways(out, &tiles) || a_mosaic(g, out, piece, owner, &tiles)) {
        return None;
    }
    let Some(surface) = wall_surface(g.p, piece, g.tol.distance, sag) else { return Some(false) };
    let r = out.len() as u32;
    let leaving: HashSet<u32> = piece.iter().map(|&t| t as u32).collect();
    for &old in &tiles {
        out[old as usize].tris.retain(|x| !leaving.contains(x));
    }
    for &t in piece {
        owner[t] = r;
    }
    out.push(Region { tris: piece.iter().map(|&t| t as u32).collect(), surface: Some(Surface::Spline(Box::new(surface))) });
    Some(true)
}

/// How many tiles make a mosaic of strips.
const MOSAIC_FROM: usize = 8;
/// How far the normals of a curved tile may spread for it to be a strip: a strip of a free form turns through 10 to 20
/// deg of its cylinder; a rounding through its quarter turn.
const STRIP_UP_TO_DEG: f64 = 60.0;

/// A MOSAIC OF STRIPS, however alike they bend: many tiles, three quarters of the piece on curved tiles each turning
/// through a narrow arc. A free form whose bend changes slowly is cut into strips of cylinders of radii within a quarter
/// of each other, which the ways of bending take for a rounding - measured on a smooth handle, 81 004 triangles of walls
/// were left as strips so. A rounding's tiles each turn through a wide arc: the quarter turn of a fillet and of the
/// sphere at its corner - a rounded bar's rim is eight such tiles and no mosaic. Flat tiles do not count either way.
fn a_mosaic(g: &Grower, out: &[Region], piece: &[usize], owner: &[u32], tiles: &HashSet<u32>) -> bool {
    if tiles.len() < MOSAIC_FROM {
        return false;
    }
    let cos_half = (STRIP_UP_TO_DEG / 2.0).to_radians().cos();
    let strip = |r: u32| {
        let region = &out[r as usize];
        if !matches!(region.surface, Some(Surface::Cylinder { .. } | Surface::Cone { .. } | Surface::Sphere { .. } | Surface::Torus { .. })) {
            return false;
        }
        let Some(mean) = region.tris.iter().fold(Vector3::zeros(), |m, &t| m + g.normal(t as usize) * g.area(t as usize)).try_normalize(1e-300) else { return false };
        region.tris.iter().all(|&t| g.normal(t as usize).dot(&mean) >= cos_half)
    };
    let strips: HashSet<u32> = tiles.iter().copied().filter(|&r| strip(r)).collect();
    2 * piece.iter().filter(|&&t| strips.contains(&owner[t])).count() >= piece.len()
}

/// A WALL BENDS MANY WAYS; a rounding does not. The strips a free form is cut into lie on cylinders and spheres of radii
/// that drift from strip to strip, while a rounding's cylinders and its corner spheres share one radius and are faces of
/// the part as they stand - measured on a bar rounded R2, a wall taken across them lost its faces.
fn bends_many_ways(out: &[Region], tiles: &HashSet<u32>) -> bool {
    let mut bends: Vec<f64> = tiles
        .iter()
        .filter_map(|&r| match out[r as usize].surface {
            Some(Surface::Cylinder { radius, .. }) | Some(Surface::Sphere { radius, .. }) => Some(radius),
            Some(Surface::Torus { minor, .. }) => Some(minor),
            Some(Surface::Cone { half_angle, .. }) => Some(1.0 / half_angle.tan().abs().max(1e-9)),
            _ => None,
        })
        .collect();
    bends.sort_by(f64::total_cmp);
    bends.windows(2).filter(|w| w[1] > 1.25 * w[0]).count() + usize::from(!bends.is_empty()) >= BENDS_FROM
}

/// The one B-spline surface holding `piece` within `tol`: through the corners of its triangles, and through their middles
/// too - a big flat triangle has corners only at its ends, and a surface through the corners alone bulged between them
/// (measured on a cube with a dent in its top, 37 mm^3 of a flat face gone); a middle stands off a curved wall by the
/// mesh's own sag, `sag`.
fn wall_surface(p: &Prepared, piece: &[usize], tol: f64, sag: f64) -> Option<crate::bspline::BSpline> {
    let corner = |t: usize, k: usize| {
        let v = p.mesh.verts[p.mesh.tris[t][k] as usize];
        Vector3::new(v.x, v.y, v.z)
    };
    let normal = |t: usize| Vector3::from(p.normals[t]);
    let area = |t: usize| (corner(t, 1) - corner(t, 0)).cross(&(corner(t, 2) - corner(t, 0))).norm() * 0.5;
    let mut at: HashMap<u32, Vector3<f64>> = HashMap::new();
    for &t in piece {
        for &v in &p.mesh.tris[t] {
            *at.entry(v).or_insert_with(Vector3::zeros) += normal(t) * area(t);
        }
    }
    let mut corners: Vec<u32> = at.keys().copied().collect();
    corners.sort_unstable();
    let mut pts: Vec<[f64; 3]> = corners
        .iter()
        .map(|&v| {
            let q = p.mesh.verts[v as usize];
            [q.x, q.y, q.z]
        })
        .collect();
    let mut nrm: Vec<[f64; 3]> = corners.iter().map(|v| at[v].try_normalize(1e-300).map(Into::into).unwrap_or([0.0, 0.0, 1.0])).collect();
    // A BIG TRIANGLE IS SAMPLED ACROSS, not at its middle alone: a surface held at its corners and middle still bowed
    // between them - measured on a cube with a dent in its top, 69 mm^3 of a flat face gone. Points every fortieth of
    // the piece's extent, at most 50 000 of them.
    let (lo, hi) = pts.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |(lo, hi), q| ([lo[0].min(q[0]), lo[1].min(q[1]), lo[2].min(q[2])], [hi[0].max(q[0]), hi[1].max(q[1]), hi[2].max(q[2])]));
    let extent = ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2)).sqrt();
    let step = (extent / 40.0).max(1e-9);
    for &t in piece {
        let (a, b, c) = (corner(t, 0), corner(t, 1), corner(t, 2));
        let longest = (b - a).norm().max((c - b).norm()).max((a - c).norm());
        let k = ((longest / step).ceil() as usize).clamp(1, 16);
        for i in 0..=k {
            for j in 0..=(k - i) {
                let (u, v) = (i as f64 / k as f64, j as f64 / k as f64);
                if k > 1 && (i + j == 0 || i == k || j == k) {
                    continue; // the corners are in already
                }
                if k == 1 && !(i == 0 && j == 0) {
                    continue;
                }
                let q = if k == 1 { (a + b + c) / 3.0 } else { a + (b - a) * u + (c - a) * v };
                pts.push(q.into());
                nrm.push(normal(t).into());
            }
        }
        if pts.len() > 50_000 {
            break;
        }
    }
    // the corners within the tolerance and the mesh's sag, a point amid a triangle within twice the sag: it stands off
    // the wall by the triangle's own sag besides
    crate::bspline::fit_held(&pts, &nrm, corners.len(), tol + sag, tol + 2.0 * sag).map(|f| f.surface)
}

/// THE FREE FORM OF A REGION WHOSE PRIMITIVE MADE NO FACE: one B-spline surface over its triangles, held within the
/// distance tolerance plus the mesh's own sag (`sag`, from `chord_deflection`); `None` where one surface does not hold
/// them. A face on a primitive fails where its strip of the primitive is trimmed by curves that leave the surface; a
/// surface laid over the region's own points carries the region's edges by construction.
pub fn free_form_of(p: &Prepared, tris: &[u32], tol: &Tolerance, sag: f64) -> Option<Surface> {
    let piece: Vec<usize> = tris.iter().map(|&t| t as usize).collect();
    wall_surface(p, &piece, tol.distance, sag).map(|b| Surface::Spline(Box::new(b)))
}

/// The pieces of `tris` that hang together across smooth sides: a half cut off by a straight line can fall apart, and
/// a face built on the loops of a region in two pieces took the wrong one (0.09 mm^2 of a region of 13).
fn connected(g: &Grower, tris: &[usize]) -> Vec<Vec<usize>> {
    let inside: HashSet<usize> = tris.iter().copied().collect();
    let mut seen: HashSet<usize> = HashSet::new();
    let mut out = Vec::new();
    for &s in tris {
        if !seen.insert(s) {
            continue;
        }
        let (mut part, mut queue) = (vec![s], VecDeque::from([s]));
        while let Some(t) = queue.pop_front() {
            for &u in &g.p.neighbours[t] {
                let u = u as usize;
                if u as u32 != NO_NEIGHBOUR && inside.contains(&u) && !g.sharp(t, u) && seen.insert(u) {
                    part.push(u);
                    queue.push_back(u);
                }
            }
        }
        out.push(part);
    }
    out
}

/// `piece` cut in two across its longest extent, laid flat on the plane square to its mean normal: the triangles whose
/// middles fall below the median go one way, the rest the other.
fn halves(g: &Grower, piece: &[usize]) -> [Vec<usize>; 2] {
    let mean = piece.iter().fold(Vector3::zeros(), |m, &t| m + g.normal(t) * g.area(t)).try_normalize(1e-300).unwrap_or_else(Vector3::z);
    let helper = if mean.x.abs() < 0.9 { Vector3::x() } else { Vector3::y() };
    let e1 = mean.cross(&helper).normalize();
    let e2 = mean.cross(&e1);
    let flat: Vec<(f64, f64)> = piece
        .iter()
        .map(|&t| {
            let c = g.centroid(t);
            (c.dot(&e1), c.dot(&e2))
        })
        .collect();
    let span = |k: fn(&(f64, f64)) -> f64| flat.iter().map(k).fold(f64::MAX, f64::min) - flat.iter().map(k).fold(f64::MIN, f64::max);
    let along: fn(&(f64, f64)) -> f64 = if span(|q| q.0).abs() >= span(|q| q.1).abs() { |q| q.0 } else { |q| q.1 };
    let mut keys: Vec<f64> = flat.iter().map(along).collect();
    let mid = keys.len() / 2;
    let median = *keys.select_nth_unstable_by(mid, f64::total_cmp).1;
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for (&t, q) in piece.iter().zip(&flat) {
        if along(q) < median {
            a.push(t)
        } else {
            b.push(t)
        }
    }
    [a, b]
}

/// A FACE MAY NOT WIND ROUND ITS AXIS. The crest of a thread is its cylinder less a groove wound round it, and a face
/// bounded so is beyond the kernel: measured on an M10 x 1.5 along 6 mm of a rod of 12, the face built on the crest's
/// cylinder took 101 of its 212 mm^2 and the crest went back to the mesh; cutting a whole band of the cylinder by the
/// loop's edges came apart as well. So a region on a surface turned about an axis, one of whose loops follows the axis
/// round a turn more than it ends up turned - a groove's hole comes back where it began, a groove running out through
/// the rim goes round once - is cut by the plane through the axis into its two halves, and each piece of a half is a
/// region of its own on the same surface: no loop within half a turn can wind.
fn unwind(g: &Grower, out: &mut Vec<Region>, owner: &mut [u32]) {
    for r in 0..out.len() {
        let (on, a) = match out[r].surface {
            Some(Surface::Cylinder { point, axis, .. }) | Some(Surface::Torus { center: point, axis, .. }) | Some(Surface::Cone { apex: point, axis, .. }) => {
                (Vector3::from(point), Vector3::from(axis))
            }
            _ => continue,
        };
        let (x, y) = across(&a);
        let angle = |v: &Vector3<f64>| (v - on).dot(&y).atan2((v - on).dot(&x));
        // the region's border, each side from the corner it leaves to the one it reaches, chained into loops
        let mut ahead: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        for &t in &out[r].tris {
            let corners = g.p.mesh.tris[t as usize];
            for k in 0..3 {
                let u = g.p.neighbours[t as usize][k];
                if u == NO_NEIGHBOUR || owner[u as usize] as usize != r {
                    ahead.entry(corners[k]).or_default().push(corners[(k + 1) % 3]);
                }
            }
        }
        let at = |i: u32| {
            let v = g.p.mesh.verts[i as usize];
            Vector3::new(v.x, v.y, v.z)
        };
        let mut wound = false;
        let mut starts: Vec<u32> = ahead.keys().copied().collect();
        starts.sort_unstable();
        for start in starts {
            while !wound && ahead.get(&start).is_some_and(|n| !n.is_empty()) {
                let (mut here, mut turned, mut lo, mut hi) = (start, 0.0f64, 0.0f64, 0.0f64);
                let mut before = angle(&at(start));
                while let Some(next) = ahead.get_mut(&here).and_then(|n| n.pop()) {
                    let now = angle(&at(next));
                    turned += (now - before + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI;
                    (lo, hi, before, here) = (lo.min(turned), hi.max(turned), now, next);
                    if here == start {
                        break;
                    }
                }
                wound = hi - lo > turned.abs() + 2.0 * std::f64::consts::PI;
            }
        }
        if !wound {
            continue;
        }
        // the halves, and the pieces of each: triangles joined across a side, their middles on one side of the plane
        let half: std::collections::HashMap<u32, bool> = out[r].tris.iter().map(|&t| (t, angle(&g.centroid(t as usize)) >= 0.0)).collect();
        let mut seen: HashSet<u32> = HashSet::new();
        let mut pieces: Vec<Vec<u32>> = Vec::new();
        for &t in &out[r].tris {
            if !seen.insert(t) {
                continue;
            }
            let (mut piece, mut queue) = (vec![t], VecDeque::from([t]));
            while let Some(s) = queue.pop_front() {
                for &u in &g.p.neighbours[s as usize] {
                    if u != NO_NEIGHBOUR && half.get(&u) == Some(&half[&t]) && seen.insert(u) {
                        piece.push(u);
                        queue.push_back(u);
                    }
                }
            }
            pieces.push(piece);
        }
        let surface = out[r].surface.clone();
        let mut pieces = pieces.into_iter();
        out[r].tris = pieces.next().unwrap_or_default();
        for piece in pieces {
            for &t in &piece {
                owner[t as usize] = out.len() as u32;
            }
            out.push(Region { tris: piece, surface: surface.clone() });
        }
    }
}

/// A SPRING'S WIRE IS ONE COIL, NOT A MOSAIC. A round wire swept along a helix is no surface of revolution, and it came
/// apart into tiles of tori, spheres and cylinders: measured on three turns of radius 10 rising 4, wire 1, 1 768
/// regions and 4 993 faces. The tiles joined across edges that are not sharp are taken together; a surface carried into
/// itself by a screw motion has every normal square to that motion's velocity, a x (x - p) + h a, and that is linear in
/// the normals' lines - (n, x * n) - so the axis and the rise come from the least eigenvector. The coil about them is
/// fitted to the corners, and taken where every triangle lies on it.
fn coiled(g: &Grower, out: &mut [Region], owner: &mut [u32]) {
    let tile = |r: &Region| match &r.surface {
        None => !r.tris.is_empty(),
        Some(Surface::Free { .. } | Surface::Spline(_) | Surface::Helix { .. } | Surface::RoundHelix { .. } | Surface::Coil { .. }) => false,
        Some(Surface::Plane { .. }) => !r.tris.is_empty() && r.tris.len() <= TILE_UP_TO,
        Some(_) => !r.tris.is_empty(),
    };
    let n = out.len();
    let mut up: Vec<usize> = (0..n).collect();
    fn root(up: &mut [usize], mut x: usize) -> usize {
        while up[x] != x {
            up[x] = up[up[x]];
            x = up[x];
        }
        x
    }
    for r in 0..n {
        if !tile(&out[r]) {
            continue;
        }
        for &t in &out[r].tris {
            for &u in &g.p.neighbours[t as usize] {
                if u == NO_NEIGHBOUR || g.sharp(t as usize, u as usize) {
                    continue;
                }
                let o = owner[u as usize] as usize;
                if o != r && tile(&out[o]) {
                    let (a, b) = (root(&mut up, r), root(&mut up, o));
                    up[a] = b;
                }
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for (r, o) in out.iter().enumerate() {
        if tile(o) {
            let k = root(&mut up, r);
            groups.entry(k).or_default().push(r);
        }
    }
    let mut groups: Vec<Vec<usize>> = groups.into_values().filter(|m| m.len() >= TILES_FROM).collect();
    groups.sort();
    for members in groups {
        let tris: Vec<u32> = members.iter().flat_map(|&r| out[r].tris.iter().copied()).collect();
        let Some(coil) = coil_about(g, &tris) else { continue };
        if !tris.iter().all(|&t| g.fits(t as usize, &coil)) {
            continue;
        }
        let keep = members[0];
        for &r in &members[1..] {
            out[r].tris.clear();
            out[r].surface = None;
        }
        for &t in &tris {
            owner[t as usize] = keep as u32;
        }
        out[keep].tris = tris;
        out[keep].surface = Some(coil);
    }
}

/// The coil the triangles `tris` lie on as nearly as it can be fitted, or `None` where their normals show no screw.
fn coil_about(g: &Grower, tris: &[u32]) -> Option<Surface> {
    // the screw: the least of a . (S_mm - S_mn S_nn^-1 S_nm) a over unit a, then c = -S_nn^-1 S_nm a, rise a . c and the
    // axis through a x c
    let (mut nn, mut nm, mut mm) = (Matrix3::<f64>::zeros(), Matrix3::<f64>::zeros(), Matrix3::<f64>::zeros());
    for &t in tris {
        let t = t as usize;
        if g.sliver[t] {
            continue;
        }
        let (n, w) = (g.normal(t), g.area(t));
        let m = g.centroid(t).cross(&n);
        nn += n * n.transpose() * w;
        nm += n * m.transpose() * w;
        mm += m * m.transpose() * w;
    }
    let solve = nn.try_inverse()?;
    let (_, vec) = eigen_sorted(mm - nm.transpose() * solve * nm);
    let a = vec[0].normalize();
    let c = -(solve * nm * a);
    let rise = a.dot(&c);
    let tau = 2.0 * std::f64::consts::PI;
    if tau * rise.abs() <= 10.0 * g.tol.distance {
        return None;
    }
    let on = a.cross(&c);
    let pts = g.corners_of(tris);
    // the first coil: the wire's middle at the corners' mean distance from the axis, its height from the corners' heights
    // less the rise, averaged round the turn, and the wire the corners' mean distance from that
    let (x, y) = across(&a);
    let pitch = tau * rise;
    let (mut far, mut round) = (0.0, nalgebra::Complex::new(0.0, 0.0));
    for p in &pts {
        let d = p - on;
        let e = d - a * d.dot(&a);
        far += e.norm() / pts.len() as f64;
        let height = d.dot(&a) - rise * e.dot(&y).atan2(e.dot(&x));
        round += nalgebra::Complex::from_polar(1.0, tau * height / pitch);
    }
    let lift = round.arg() * pitch / tau;
    let first = Surface::Coil { point: on.into(), axis: a.into(), reference: x.into(), rise, radius: far, lift, wire: 0.0 };
    let reach: Vec<f64> = pts.iter().filter_map(|p| coil_at(&first, p)).map(|at| at.out.norm()).collect();
    let wire = reach.iter().sum::<f64>() / reach.len().max(1) as f64;
    // A WIRE IS ROUND BEFORE IT IS REFINED: the corners of the spring's mesh stood 0.16 % of the wire apart in their distance
    // from the first middle line, those of every smooth patch of the samples whose normals let a screw through 8 % and
    // more; refining each of those to turn it down took a mesh of hearts and gears from 0.84 s to 2.2 s to split
    let spread = (reach.iter().map(|r| (r - wire).powi(2)).sum::<f64>() / reach.len().max(1) as f64).sqrt();
    if spread > 0.04 * wire {
        return None;
    }
    // refined: the axis tilted and moved across itself, the rise, the radius, the lift and the wire
    let (u, w) = (x, y);
    let make = |q: &[f64; 8]| {
        let axis = (a + u * q[0] + w * q[1]).normalize();
        let reference = (x - axis * axis.dot(&x)).normalize();
        Surface::Coil { point: (on + u * q[2] + w * q[3]).into(), axis: axis.into(), reference: reference.into(), rise: q[4], radius: q[5], lift: q[6], wire: q[7] }
    };
    let miss = |q: &[f64; 8], v: &Vector3<f64>| {
        let off = |q: &[f64; 8]| coil_at(&make(q), v).map_or(0.0, |at| at.off);
        let now = off(q);
        let mut grad = [0.0; 8];
        for k in 0..8 {
            let step = 1e-6 * (1.0 + q[k].abs());
            let (mut up, mut down) = (*q, *q);
            up[k] += step;
            down[k] -= step;
            grad[k] = (off(&up) - off(&down)) / (2.0 * step);
        }
        (now, grad)
    };
    let mut q = [0.0, 0.0, 0.0, 0.0, rise, far, lift, wire];
    refine(&mut q, &pts, &miss, 20, 600, None, g.tol.distance * 0.1);
    (q[5] > q[7] && q[7] > 0.0).then(|| make(&q))
}

/// How many triangles a plane may have and still be a tile of a free form: a flat bit of one, not a face beside it.
const TILE_UP_TO: usize = 40;
/// How few tiles make a free form.
const TILES_FROM: usize = 6;
/// How many kinds of bending the tiles of a free form show at the least: a rounding and its corner spheres bend alike.
const BENDS_FROM: usize = 3;

/// A FREE FORM IS ONE REGION, NOT A MOSAIC. A smooth wall no plane, cylinder, cone, sphere or torus is - a loft, a
/// styled cover - is split into tiles, each within the tolerance of a surface of its own: measured on a smooth loft
/// through squares of 10, 6 and 10, 178 regions and 494 faces for the six the loft has. The tiles - curved regions and
/// small planes joined across edges that are not sharp - become one region of a free form where they are many, bend in
/// at least three different ways (a radius more than 1.25 times another's), all face one side, and no single surface
/// takes them whole. A rounded bar stays itself: its roundings and corner spheres bend alike.
fn free_form(g: &Grower, out: &mut [Region], owner: &mut [u32]) {
    let tile = |r: &Region| match &r.surface {
        None | Some(Surface::Free { .. } | Surface::Spline(_) | Surface::Helix { .. } | Surface::RoundHelix { .. } | Surface::Coil { .. }) => false,
        Some(Surface::Plane { .. }) => !r.tris.is_empty() && r.tris.len() <= TILE_UP_TO,
        Some(_) => !r.tris.is_empty(),
    };
    let n = out.len();
    let mut up: Vec<usize> = (0..n).collect();
    fn root(up: &mut [usize], mut x: usize) -> usize {
        while up[x] != x {
            up[x] = up[up[x]];
            x = up[x];
        }
        x
    }
    for r in 0..n {
        if !tile(&out[r]) {
            continue;
        }
        for &t in &out[r].tris {
            for &u in &g.p.neighbours[t as usize] {
                if u == NO_NEIGHBOUR || g.sharp(t as usize, u as usize) {
                    continue;
                }
                let o = owner[u as usize] as usize;
                if o != r && tile(&out[o]) {
                    let (a, b) = (root(&mut up, r), root(&mut up, o));
                    up[a] = b;
                }
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for (r, o) in out.iter().enumerate() {
        if tile(o) {
            let k = root(&mut up, r);
            groups.entry(k).or_default().push(r);
        }
    }
    let mut groups: Vec<Vec<usize>> = groups.into_values().filter(|m| m.len() >= TILES_FROM).collect();
    groups.sort();
    for members in groups {
        // the ways the tiles bend: radii within 1.25 times of one another are one way
        let mut bends: Vec<f64> = members
            .iter()
            .filter_map(|&r| match out[r].surface {
                Some(Surface::Cylinder { radius, .. }) | Some(Surface::Sphere { radius, .. }) => Some(radius),
                Some(Surface::Torus { minor, .. }) => Some(minor),
                Some(Surface::Cone { half_angle, .. }) => Some(1.0 / half_angle.tan().abs().max(1e-9)),
                _ => None,
            })
            .collect();
        bends.sort_by(f64::total_cmp);
        let ways = bends.windows(2).filter(|w| w[1] > 1.25 * w[0]).count() + usize::from(!bends.is_empty());
        if ways < BENDS_FROM {
            continue;
        }
        let tris: Vec<u32> = members.iter().flat_map(|&r| out[r].tris.iter().copied()).collect();
        let Some(mean) = tris.iter().fold(Vector3::zeros(), |m, &t| m + g.normal(t as usize) * g.area(t as usize)).try_normalize(1e-12) else { continue };
        if tris.iter().any(|&t| !g.sliver[t as usize] && g.normal(t as usize).dot(&mean) <= 0.0) {
            continue;
        }
        let sample = Sample::of(g, &tris);
        if [Kind::Plane, Kind::Cylinder, Kind::Cone, Kind::Sphere, Kind::Torus].into_iter().any(|k| g.fit_sample(&sample, k).is_some_and(|s| tris.iter().all(|&t| g.fits(t as usize, &s)))) {
            continue;
        }
        let keep = members[0];
        for &r in &members[1..] {
            out[r].tris.clear();
            out[r].surface = None;
        }
        for &t in &tris {
            owner[t as usize] = keep as u32;
        }
        out[keep].tris = tris;
        out[keep].surface = Some(Surface::Free { normal: mean.into() });
    }
}

/// REGIONS THAT LIE ON ONE SURFACE TOGETHER BECOME ONE.
///
/// A seed sees a few rings around it, and on a torus that is a narrow band: measured on a torus of radii 20 and 5,
/// the seeds grew bands of cylinders (radius 5.003) and spheres (5.078) and not one torus - the tube's centre points
/// of one band lie on too short an arc to fit a circle to. Two neighbouring bands span twice the arc. So curved
/// neighbours are joined while their union fits one surface - the kind of either, a cone or a torus - with every
/// triangle on it; the widest regions are joined first. An emptied region is dropped at the end.
fn merge(g: &Grower, out: &mut [Region], owner: &mut [u32]) {
    let curved = |r: &Region| r.tris.len() >= MERGE_FROM && matches!(r.surface, Some(Surface::Cylinder { .. } | Surface::Sphere { .. } | Surface::Cone { .. } | Surface::Torus { .. }));
    // A JOIN IS TRIED ONLY WHERE IT CAN BE TRUE. On a 225 154-triangle organic mesh of 28 000 regions the joins took
    // 186 s when every pair was tried in every pass and refined to the end.
    // after the first pass only a pair of which one side has changed can join
    let mut changed: Option<HashSet<usize>> = None;
    loop {
        let mut now: HashSet<usize> = HashSet::new();
        let mut order: Vec<usize> = (0..out.len()).filter(|&r| curved(&out[r])).collect();
        order.sort_by_key(|&r| std::cmp::Reverse(out[r].tris.len()));
        for a in order {
            let mut tried: HashSet<usize> = HashSet::new();
            // the region keeps taking neighbours while one fits, so a torus grows band by band in one pass
            'more: loop {
                if !curved(&out[a]) {
                    break; // emptied by an earlier join
                }
                let mut near: Vec<usize> = Vec::new();
                for &t in &out[a].tris {
                    for &u in &g.p.neighbours[t as usize] {
                        if u == NO_NEIGHBOUR || g.sharp(t as usize, u as usize) {
                            continue;
                        }
                        let b = owner[u as usize] as usize;
                        let fresh = changed.as_ref().is_none_or(|c| c.contains(&a) || c.contains(&b));
                        if b != a && fresh && !near.contains(&b) && !tried.contains(&b) && curved(&out[b]) && alike(&out[a].surface, &out[b].surface) {
                            near.push(b);
                        }
                    }
                }
                for b in near {
                    tried.insert(b);
                    // THE CHEAP WAY FIRST: a neighbour lying on the surface already found joins it as it is. Once a torus
                    // stands from its first bands, the rest of them do, and refitting the union for each one cost 6 ms
                    // a band on a print program's torus.
                    if out[a].surface.as_ref().is_some_and(|s| out[b].tris.iter().all(|&t| g.fits(t as usize, s))) {
                        let taken = std::mem::take(&mut out[b].tris);
                        for &t in &taken {
                            owner[t as usize] = a as u32;
                        }
                        out[a].tris.extend(taken);
                        out[b].surface = None;
                        now.insert(a);
                        continue 'more;
                    }
                    let union: Vec<u32> = out[a].tris.iter().chain(&out[b].tris).copied().collect();
                    let mut kinds: Vec<Kind> = Vec::new();
                    for k in [out[a].surface.as_ref().map(kind_of), out[b].surface.as_ref().map(kind_of), Some(Kind::Cone), Some(Kind::Torus)].into_iter().flatten() {
                        if !kinds.iter().any(|x| std::mem::discriminant(x) == std::mem::discriminant(&k)) {
                            kinds.push(k);
                        }
                    }
                    let sample = Sample::of(g, &union);
                    let Some(s) = kinds.into_iter().find_map(|k| g.fit_sample(&sample, k).filter(|s| union.iter().all(|&t| g.fits(t as usize, s)))) else { continue };
                    for &t in &out[b].tris {
                        owner[t as usize] = a as u32;
                    }
                    out[b].tris.clear();
                    out[b].surface = None;
                    out[a].tris = union;
                    out[a].surface = Some(s);
                    now.insert(a);
                    continue 'more; // a neighbour turned down gets its second chance in the next pass
                }
                break;
            }
        }
        if now.is_empty() {
            break;
        }
        changed = Some(now);
    }
}

/// A REGION LYING WHOLLY ON A LARGER NEIGHBOUR'S SURFACE joins it, the smallest first. Measured on the torus: after
/// the joins two single triangles stood alone as planes, and on a print program's torus a strip of flat pairs along
/// the inner equator - each lying on the torus it was cut from. A plane face beside a rounding does not lie on its
/// cylinder and stays.
///
/// IT IS REPEATED UNTIL NOTHING MORE JOINS. Measured on a cone: a ring of single triangles along the base touched only
/// the cap and regions of three, which joined the cone after the singles had been looked at.
fn absorb(g: &Grower, out: &mut [Region], owner: &mut [u32]) {
    loop {
        if !absorb_once(g, out, owner) {
            break;
        }
    }
}

/// One pass of `absorb`, the smallest regions first; whether anything joined.
fn absorb_once(g: &Grower, out: &mut [Region], owner: &mut [u32]) -> bool {
    let mut joined = false;
    let mut order: Vec<usize> = (0..out.len()).filter(|&r| !out[r].tris.is_empty()).collect();
    order.sort_by_key(|&r| out[r].tris.len());
    for r in order {
        if out[r].tris.is_empty() {
            continue;
        }
        let tris = out[r].tris.clone();
        let mut near: Vec<usize> =
            tris.iter().flat_map(|&t| g.p.neighbours[t as usize]).filter(|&u| u != NO_NEIGHBOUR).map(|u| owner[u as usize] as usize).filter(|&h| h != r && out[h].tris.len() > tris.len()).collect();
        near.sort_unstable();
        near.dedup();
        // A TRIANGLE WHOSE NORMAL THE TOLERANCE CAN TURN PAST THE ANGLE is judged by its corners alone: at the owner's
        // screw at a hundred times the tolerance, 1 677 triangles with sides of 0.01 mm stood alone, turned 33 to 44 deg
        // from neighbours whose surface their corners lay on. Only in joining: a seed still needs a normal to trust.
        let lies = |t: u32, s: &Surface| g.fits(t as usize, s) || (g.corners_on(t as usize, s) && !g.normal_trusted(t as usize));
        let home = near.into_iter().find(|&h| out[h].surface.as_ref().is_some_and(|s| tris.iter().all(|&t| lies(t, s))));
        if let Some(h) = home {
            for &t in &tris {
                owner[t as usize] = h as u32;
            }
            out[h].tris.extend(tris);
            out[r].tris.clear();
            out[r].surface = None;
            joined = true;
        }
    }
    joined
}

/// THE AXIS A SURFACE OF REVOLUTION SHARES WITH ITS NEIGHBOURS.
///
/// A band a few triangles wide - the flank of a thread drawn as rings - does not show its own axis: its normals barely
/// part along it, and a sphere through its two edges fits its corners as closely as the true cone does (two coaxial
/// circles always lie on one sphere). Measured on a ring thread from our kernel, ten rings and 43 faces: every crest
/// and root came as one cylinder, and every flank came apart into 117 - 169 regions of cylinders, planes and spheres.
/// So the axes of the cylinders found are taken as axes the regions around them may turn about: neighbours whose
/// corners, seen as (height along the axis, distance from it), lie on one straight line join as a cone - or a
/// cylinder, or a plane - about that axis. A straight line is taken before a curve: with no corner between a band's
/// two edges nothing shows it bends. A sphere or a torus about the axis lies on no straight line and stays as it is.
fn coaxial(g: &Grower, out: &mut [Region], owner: &mut [u32]) {
    for (on, a) in &cylinder_axes(g, out) {
        // THE FAMILY OF THE AXIS, flooded from its cylinders through the regions beside them - across any edge, a
        // sharp one too - that lie on a straight line about it on their own. Measured on the tensioner screw (386 924
        // triangles): trying every region against its neighbours for each of the axes took the split from 10.7 s to
        // 808 s; the family of a thread's axis is the thread.
        let family = axis_family(g, out, owner, on, a, |tris| straight_about(g, tris, on, a).is_some());
        // A MEMBER OF THE FAMILY BENT BY ITS OWN FIT TAKES THE STRAIGHT SURFACE IT LIES ON. A band whose triangles run
        // rim to rim has its corners on two circles about the axis, and any two such circles lie on a sphere as exactly
        // as on a cone: a chamfer of 0.5 at 45 deg on a washer's rim came in as a sphere of 13.79, and across its sharp
        // edges nothing joined it to be refitted. Lying on a straight line about the axis, it is the cone.
        for &r in &family {
            if matches!(out[r].surface, Some(Surface::Sphere { .. } | Surface::Torus { .. })) {
                if let Some(straight) = straight_about(g, &out[r].tris, on, a) {
                    out[r].surface = Some(straight);
                }
            }
        }
        // neighbours of the family across an edge that is not sharp join, the widest first: at once where the one
        // lies on the other's surface already, and otherwise where their union lies on one straight line
        loop {
            let mut joined = false;
            let mut order: Vec<usize> = family.iter().copied().filter(|&r| !out[r].tris.is_empty()).collect();
            order.sort_by_key(|&r| (std::cmp::Reverse(out[r].tris.len()), r));
            for ra in order {
                'more: loop {
                    if out[ra].tris.is_empty() {
                        break;
                    }
                    let mut near: Vec<usize> = Vec::new();
                    for &t in &out[ra].tris {
                        for &u in &g.p.neighbours[t as usize] {
                            if u == NO_NEIGHBOUR || g.sharp(t as usize, u as usize) {
                                continue;
                            }
                            let rb = owner[u as usize] as usize;
                            if rb != ra && family.contains(&rb) && !near.contains(&rb) {
                                near.push(rb);
                            }
                        }
                    }
                    for rb in near {
                        let fits_as_is = out[ra].surface.as_ref().filter(|s| straight_kind(s)).is_some_and(|s| out[rb].tris.iter().all(|&t| g.fits(t as usize, s)));
                        let union: Vec<u32> = out[ra].tris.iter().chain(&out[rb].tris).copied().collect();
                        let s = if fits_as_is { out[ra].surface.clone() } else { straight_about(g, &union, on, a) };
                        let Some(s) = s else { continue };
                        for &t in &out[rb].tris {
                            owner[t as usize] = ra as u32;
                        }
                        out[rb].tris.clear();
                        out[rb].surface = None;
                        out[ra].tris = union;
                        out[ra].surface = Some(s);
                        joined = true;
                        continue 'more;
                    }
                    break;
                }
            }
            if !joined {
                break;
            }
        }
    }
}

/// The axes of the cylinders found, one per line, the widest cylinder's first: two lie on one line when they are parallel
/// to a thousandth of a radian and a point of one is within ten distances of the other.
fn cylinder_axes(g: &Grower, out: &[Region]) -> Vec<(Vector3<f64>, Vector3<f64>)> {
    let mut axes: Vec<(Vector3<f64>, Vector3<f64>)> = Vec::new();
    let mut cylinders: Vec<&Region> = out.iter().filter(|r| r.tris.len() >= MERGE_FROM && matches!(r.surface, Some(Surface::Cylinder { .. }))).collect();
    cylinders.sort_by_key(|r| std::cmp::Reverse(r.tris.len()));
    for r in cylinders {
        let Some(Surface::Cylinder { point, axis, .. }) = r.surface else { continue };
        let (p, a) = (Vector3::from(point), Vector3::from(axis));
        let known = axes.iter().any(|(q, b)| a.cross(b).norm() < 1e-3 && ((p - q) - b * (p - q).dot(b)).norm() < 10.0 * g.tol.distance);
        if !known {
            axes.push((p, a));
        }
    }
    axes
}

/// The regions flooded from the cylinders on the axis through `on` along `a`, across any edge, through the regions that
/// `belongs` takes.
fn axis_family(g: &Grower, out: &[Region], owner: &[u32], on: &Vector3<f64>, a: &Vector3<f64>, belongs: impl Fn(&[u32]) -> bool) -> HashSet<usize> {
    let on_axis = |r: &Region| match r.surface {
        Some(Surface::Cylinder { point, axis, .. }) => {
            let (p, b) = (Vector3::from(point), Vector3::from(axis));
            b.cross(a).norm() < 1e-3 && ((p - on) - a * (p - on).dot(a)).norm() < 10.0 * g.tol.distance
        }
        _ => false,
    };
    let mut family: HashSet<usize> = (0..out.len()).filter(|&r| out[r].tris.len() >= MERGE_FROM && on_axis(&out[r])).collect();
    let mut looked: HashSet<usize> = family.clone();
    let mut queue: VecDeque<usize> = family.iter().copied().collect();
    while let Some(r) = queue.pop_front() {
        let mut next: Vec<usize> = Vec::new();
        for &t in &out[r].tris {
            for &u in &g.p.neighbours[t as usize] {
                if u != NO_NEIGHBOUR && !looked.contains(&(owner[u as usize] as usize)) && !next.contains(&(owner[u as usize] as usize)) {
                    next.push(owner[u as usize] as usize);
                }
            }
        }
        for rb in next {
            looked.insert(rb);
            if belongs(&out[rb].tris) {
                family.insert(rb);
                queue.push_back(rb);
            }
        }
    }
    family
}

/// Whether the corners of `tris` go more than half way round the axis through `on` along `a`: no gap between their angles
/// round it is as wide as half a turn.
fn goes_round(g: &Grower, tris: &[u32], on: &Vector3<f64>, a: &Vector3<f64>) -> bool {
    let (x, y) = across(a);
    let mut angles: Vec<f64> = g.corners_of(tris).iter().map(|c| (c - on).dot(&y).atan2((c - on).dot(&x))).collect();
    angles.sort_by(f64::total_cmp);
    let (Some(&first), Some(&last)) = (angles.first(), angles.last()) else { return false };
    let widest = angles.windows(2).map(|w| w[1] - w[0]).fold(first + 2.0 * std::f64::consts::PI - last, f64::max);
    widest < std::f64::consts::PI
}

/// Whether `s` is what a straight line about an axis gives: a cone, a cylinder or a plane.
fn straight_kind(s: &Surface) -> bool {
    matches!(s, Surface::Cone { .. } | Surface::Cylinder { .. } | Surface::Plane { .. })
}

/// The cone, cylinder or plane about the axis through `on` along `a` that the corners of `tris` lie on: their (height,
/// distance) points on one straight line - the principal direction of their scatter - each within the distance of it,
/// which is the corner's distance from the surface itself; and every triangle's normal agreeing. `None` otherwise.
fn straight_about(g: &Grower, tris: &[u32], on: &Vector3<f64>, a: &Vector3<f64>) -> Option<Surface> {
    let zr = profile(&g.corners_of(tris), a, on);
    let n = zr.len() as f64;
    if zr.len() < 3 {
        return None;
    }
    let (mz, mr) = (zr.iter().map(|x| x.0).sum::<f64>() / n, zr.iter().map(|x| x.1).sum::<f64>() / n);
    let (szz, szr, srr) = zr.iter().fold((0.0, 0.0, 0.0), |(zz, zrr, rr), &(z, r)| (zz + (z - mz).powi(2), zrr + (z - mz) * (r - mr), rr + (r - mr).powi(2)));
    let theta = 0.5 * (2.0 * szr).atan2(szz - srr);
    let (dz, dr) = (theta.cos(), theta.sin());
    if zr.iter().any(|&(z, r)| ((z - mz) * dr - (r - mr) * dz).abs() > g.tol.distance) {
        return None;
    }
    // A SLOPE THE REGION CANNOT SHOW IS NO SLOPE. The line of a cylinder's wall came out at 1e-6 rather than zero, and
    // the wall - measured under a sphere set 0.1 mm off its axis - stood as four cones opening from 1e7 mm below it, each
    // a cone of its own that no neighbour joined; across the region's height its radius changed by a thousandth of the
    // distance tolerance.
    let zs = zr.iter().fold(0.0f64, |zs, &(z, _)| zs.max((z - mz).abs()));
    let s = if dr.abs() * zs <= g.tol.distance * dz.abs() {
        Surface::Cylinder { point: (*on).into(), axis: (*a).into(), radius: mr }
    } else if dz.abs() <= 1e-9 * dr.abs() {
        Surface::Plane { point: (on + a * mz).into(), normal: (*a).into() }
    } else {
        // the apex where the line meets the axis, the cone opening from it towards the corners
        let z0 = mz - mr * dz / dr;
        let into = if mz > z0 { *a } else { -a };
        Surface::Cone { apex: (on + a * z0).into(), axis: into.into(), half_angle: (dr / dz).abs().atan() }
    };
    tris.iter().all(|&t| g.fits(t as usize, &s)).then_some(s)
}

/// THE FLANK OF A THREAD IS ONE HELIX, NOT A MOSAIC. A profile screwed about an axis is no surface of revolution, and its
/// flank came apart into tiles each within the tolerance of a small cylinder, plane or sphere: measured on an M10 x 1.5
/// cut by our kernel along 6 mm, 1 737 regions and 2 340 faces. Every face of such a thread is a straight piece of the
/// profile screwed along - the crest and the root cylinders about the axis the flanks turn round. So the axes of the
/// cylinders found are taken again, and the regions around them that lie on a helix about one are joined across edges
/// that are not sharp while their union lies on one helix, every triangle on it.
fn helical(g: &Grower, out: &mut [Region], owner: &mut [u32]) {
    // A THREAD'S AXIS IS SEEN ROUND: its crest or root goes all round it. An organic mesh's small cylinders give axes
    // of their own by the thousand - 8 003 on a 225 154-triangle one, and trying each took its split from 9.3 s to 21.3 s.
    let round: Vec<(Vector3<f64>, Vector3<f64>)> = cylinder_axes(g, out)
        .into_iter()
        .filter(|(on, a)| {
            out.iter().any(|r| match r.surface {
                Some(Surface::Cylinder { point, axis, .. }) if r.tris.len() >= MERGE_FROM => {
                    let (p, b) = (Vector3::from(point), Vector3::from(axis));
                    b.cross(a).norm() < 1e-3 && ((p - on) - a * (p - on).dot(a)).norm() < 10.0 * g.tol.distance && goes_round(g, &r.tris, on, a)
                }
                _ => false,
            })
        })
        .collect();
    for (on, a) in &round {
        // a region too small to show a helix of its own lets the flood through and is joined by its neighbour's
        let family = axis_family(g, out, owner, on, a, |tris| tris.len() <= 2 || straight_about(g, tris, on, a).is_some() || helix_about(g, tris, on, a, None).is_some());
        // the joins are made on a copy, each region keeping which regions it took
        let (mut work, mut whose) = (out.to_vec(), owner.to_vec());
        let mut took: Vec<Vec<usize>> = (0..out.len()).map(|r| vec![r]).collect();
        loop {
            let mut joined = false;
            let mut order: Vec<usize> = family.iter().copied().filter(|&r| work[r].tris.len() > 2).collect();
            order.sort_by_key(|&r| (std::cmp::Reverse(work[r].tris.len()), r));
            for ra in order {
                // A NEIGHBOUR TURNED DOWN GETS ITS SECOND CHANCE IN THE NEXT PASS, not after every join: tried again each
                // time - and a circle refined for each - an auger's split took 28 s, once a pass 1.8 s
                let mut tried: HashSet<usize> = HashSet::new();
                'more: loop {
                    if work[ra].tris.is_empty() {
                        break;
                    }
                    let own = match work[ra].surface {
                        Some(Surface::Helix { .. } | Surface::RoundHelix { .. }) => work[ra].surface.clone(),
                        _ => helix_about(g, &work[ra].tris, on, a, None),
                    };
                    let Some(own) = own else { break };
                    let mut near: Vec<usize> = Vec::new();
                    for &t in &work[ra].tris {
                        for &u in &g.p.neighbours[t as usize] {
                            if u == NO_NEIGHBOUR || g.sharp(t as usize, u as usize) {
                                continue;
                            }
                            let rb = whose[u as usize] as usize;
                            if rb != ra && family.contains(&rb) && !near.contains(&rb) && !tried.contains(&rb) {
                                near.push(rb);
                            }
                        }
                    }
                    for rb in near {
                        tried.insert(rb);
                        let union: Vec<u32> = work[ra].tris.iter().chain(&work[rb].tris).copied().collect();
                        let s = if work[rb].tris.iter().all(|&t| g.fits(t as usize, &own)) { Some(own.clone()) } else { helix_about(g, &union, on, a, Some(&own)) };
                        let Some(s) = s else { continue };
                        for &t in &work[rb].tris {
                            whose[t as usize] = ra as u32;
                        }
                        work[rb].tris.clear();
                        work[rb].surface = None;
                        work[ra].tris = union;
                        work[ra].surface = Some(s);
                        let moved = std::mem::take(&mut took[rb]);
                        took[ra].extend(moved);
                        joined = true;
                        continue 'more;
                    }
                    break;
                }
            }
            if !joined {
                break;
            }
        }
        // A SCREW IS SEEN BY TURNING: a helix is taken where it goes a whole turn round its axis at the least. A smooth
        // loft's tiles lie on helices too, about the axes of small cylinders among them: measured through squares of 10,
        // 6 and 10, 32 such helices of 11 - 88 triangles, each spanning 0.03 - 0.13 rad, and the walls no longer came
        // together as free forms; on an airplane's mesh one of half a turn took tiles of its free forms. A thread's flank
        // spans all its turns - 25 rad along 6 mm of an M10 x 1.5.
        for r in 0..work.len() {
            let Some(h @ (Surface::Helix { .. } | Surface::RoundHelix { .. })) = &work[r].surface else { continue };
            let (lo, hi) = g.corners_of(&work[r].tris).iter().filter_map(|c| h.turn_at([c.x, c.y, c.z])).fold((f64::MAX, f64::MIN), |(lo, hi), (t, _)| (lo.min(t), hi.max(t)));
            if hi - lo < 2.0 * std::f64::consts::PI {
                continue;
            }
            for &m in &took[r] {
                if m != r {
                    out[m].tris.clear();
                    out[m].surface = None;
                }
            }
            for &t in &work[r].tris {
                owner[t as usize] = r as u32;
            }
            out[r] = work[r].clone();
        }
    }
}

/// The helix about the axis through `on` along `a` that every triangle of `tris` lies on, or `None`.
///
/// Its line in (r, z, t) - distance, height and angle round the axis - is fitted to the corners by total least squares:
/// radial * r + axial * z - axial * rise * t = offset with (radial, axial) of unit length. Each corner's angle takes the
/// whole turns that bring it onto `guide`, the helix of a part of the union - a flank runs many turns. With no guide the
/// corners are taken within half a turn of the first: the pieces tried so are a few triangles. A helix whose rise the
/// corners cannot show across their angles is a cone or a plane.
fn helix_about(g: &Grower, tris: &[u32], on: &Vector3<f64>, a: &Vector3<f64>, guide: Option<&Surface>) -> Option<Surface> {
    let (x, y) = across(a);
    let pts = g.corners_of(tris);
    if pts.len() < 4 {
        return None;
    }
    let tau = 2.0 * std::f64::consts::PI;
    let mut rzt: Vec<(f64, f64, f64)> = pts
        .iter()
        .map(|p| {
            let d = p - on;
            let z = d.dot(a);
            let e = d - a * z;
            (e.norm(), z, e.dot(&y).atan2(e.dot(&x)))
        })
        .collect();
    match guide {
        Some(h @ (Surface::Helix { .. } | Surface::RoundHelix { .. })) => {
            for (c, p) in rzt.iter_mut().zip(&pts) {
                c.2 = helix_at(h, p)?.turn;
            }
        }
        _ => {
            let t0 = rzt[0].2;
            for c in &mut rzt {
                c.2 = t0 + (c.2 - t0 + std::f64::consts::PI).rem_euclid(tau) - std::f64::consts::PI;
            }
        }
    }
    let n = rzt.len() as f64;
    let (mr, mz, mt) = rzt.iter().fold((0.0, 0.0, 0.0), |m, c| (m.0 + c.0 / n, m.1 + c.1 / n, m.2 + c.2 / n));
    let (mut srr, mut srz, mut szz, mut srt, mut szt, mut stt) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for &(r, z, t) in &rzt {
        let (r, z, t) = (r - mr, z - mz, t - mt);
        srr += r * r;
        srz += r * z;
        szz += z * z;
        srt += r * t;
        szt += z * t;
        stt += t * t;
    }
    // the angle the corners span shows the screw; along a hair of it any rise fits
    if stt < n * 1e-6 {
        return None;
    }
    // the rise taken out, what is left of the scatter of (r, z): its least direction is the profile's normal
    let m = nalgebra::Matrix2::new(srr - srt * srt / stt, srz - srt * szt / stt, srz - srt * szt / stt, szz - szt * szt / stt);
    let e = nalgebra::SymmetricEigen::new(m);
    let least = if e.eigenvalues[0] <= e.eigenvalues[1] { 0 } else { 1 };
    let (mut radial, mut axial) = (e.eigenvectors[(0, least)], e.eigenvectors[(1, least)]);
    if axial < 0.0 {
        (radial, axial) = (-radial, -axial);
    }
    let (lo, hi) = rzt.iter().fold((f64::MAX, f64::MIN), |(lo, hi), c| (lo.min(c.2), hi.max(c.2)));
    if axial > 1e-9 {
        let turned = (radial * srt + axial * szt) / stt;
        let rise = turned / axial;
        // A HELIX WHOSE TURNS LIE ON ONE ANOTHER IS A CYLINDER: its turns stand 2 pi x axial x rise apart across it, and
        // a loft's wall came as one with axial 9e-9 and a rise of 354 - turns 2e-5 mm apart, and 716 rad to a few triangles
        if (rise * (hi - lo)).abs() > g.tol.distance && tau * (axial * rise).abs() > 10.0 * g.tol.distance {
            let s = Surface::Helix { point: (*on).into(), axis: (*a).into(), reference: x.into(), rise, radial, axial, offset: radial * mr + axial * mz - turned * mt };
            if tris.iter().all(|&t| g.fits(t as usize, &s)) {
                return Some(s);
            }
        }
    }
    // a union grown on a straight helix stays straight: refining a circle for every neighbour turned down took an auger's
    // split from 1.8 s to 2.2 s
    if matches!(guide, Some(Surface::Helix { .. })) {
        return None;
    }
    round_about(g, tris, &pts, &rzt, on, a, guide)
}

/// THE PROFILE MAY BE A CIRCLE. An auger's flight rounded by 0.8 our kernel draws in five straight pieces, but the mesh's
/// corners lie on the circle through them, and its pieces of a few triangles each took two of them: measured, they
/// missed every straight helix by 0.023 mm. The rise is the guide's, or the one the triangles' normals show - a screwed
/// surface's normal stands square to the screw's motion, a x (v - on) + rise a; the corners less the rise are fitted a
/// circle in (r, z - rise * t) algebraically, and all four numbers refined on the corners.
fn round_about(g: &Grower, tris: &[u32], pts: &[Vector3<f64>], rzt: &[(f64, f64, f64)], on: &Vector3<f64>, a: &Vector3<f64>, guide: Option<&Surface>) -> Option<Surface> {
    if pts.len() < 6 {
        return None;
    }
    let (lo, hi) = rzt.iter().fold((f64::MAX, f64::MIN), |(lo, hi), c| (lo.min(c.2), hi.max(c.2)));
    let span = hi - lo;
    let tau = 2.0 * std::f64::consts::PI;
    let rise = match guide {
        Some(Surface::Helix { rise, .. } | Surface::RoundHelix { rise, .. }) => *rise,
        _ => {
            let (mut turned, mut along) = (0.0, 0.0);
            for &t in tris {
                let t = t as usize;
                if g.sliver[t] {
                    continue;
                }
                let (n, w) = (g.normal(t), g.area(t));
                turned += w * n.dot(a) * n.dot(&a.cross(&(g.centroid(t) - on)));
                along += w * n.dot(a) * n.dot(a);
            }
            if along < 1e-12 {
                return None;
            }
            -turned / along
        }
    };
    let pitch = tau * rise;
    if (rise * span).abs() <= g.tol.distance || pitch.abs() <= 10.0 * g.tol.distance {
        return None;
    }
    let first = rzt[0].1 - rise * rzt[0].2;
    let profile: Vec<(f64, f64)> = rzt
        .iter()
        .map(|&(r, z, t)| {
            let lifted = z - rise * t;
            (r, lifted - pitch * ((lifted - first) / pitch).round())
        })
        .collect();
    let (mut m, mut v) = (Matrix3::<f64>::zeros(), Vector3::<f64>::zeros());
    for &(r, q) in &profile {
        let row = Vector3::new(r, q, 1.0);
        m += row * row.transpose();
        v -= row * (r * r + q * q);
    }
    let k = m.lu().solve(&v)?;
    let (middle, height) = (-k.x / 2.0, -k.y / 2.0);
    let squared = middle * middle + height * height - k.z;
    if squared.is_nan() || squared <= 0.0 {
        return None;
    }

    let x = across(a).0;
    let make = |q: &[f64; 4]| Surface::RoundHelix { point: (*on).into(), axis: (*a).into(), reference: x.into(), rise: q[0], middle: q[1], height: q[2], round: q[3] };
    // the miss and its derivatives written out: off = (apart - round) / |normal|, apart the distance from the circle's
    // middle in (r, z - rise * t); the slow change of |normal| with the numbers is left out, which Gauss-Newton bears.
    // By small steps each miss cost nine, and a mesh of hearts and gears took 1.20 s to split against 0.84 s.
    let miss = |q: &[f64; 4], v: &Vector3<f64>| {
        let Some(h) = helix_at(&make(q), v) else { return (0.0, [0.0; 4]) };
        let d = v - on;
        let z = d.dot(a);
        let far = (d - a * z).norm();
        let (dr, dz) = (far - q[1], z - q[0] * h.turn - q[2]);
        let apart = dr.hypot(dz).max(1e-300);
        let scale = 1.0 / h.normal.norm().max(1e-300);
        (h.off, [-dz / apart * h.turn * scale, -dr / apart * scale, -dz / apart * scale, -scale])
    };
    let mut q = [rise, middle, height, squared.sqrt()];
    refine(&mut q, pts, &miss, 20, 600, None, g.tol.distance * 0.1);
    // a circle within the pitch and inside its distance from the axis, the tolerance at the least
    if !(q[3] > g.tol.distance && 2.0 * q[3] < (tau * q[0]).abs() && q[3] < q[1]) {
        return None;
    }
    let s = make(&q);
    tris.iter().all(|&t| g.fits(t as usize, &s)).then_some(s)
}

/// How few triangles a region may have and still be joined. Measured on a 225 154-triangle organic mesh: most of its
/// 28 000 regions are of three to five, none of them lies on a torus or a cone with a neighbour, and trying them all
/// took the joins to 95 s; a band of a torus from a print program is some forty.
const MERGE_FROM: usize = 8;

/// Whether two surfaces bend alike: the radii of two bands of one torus - a cylinder's and a sphere's - differ by a
/// hundredth, of an organic mesh's neighbours by anything. A cone bends along its length, so it is always tried.
fn alike(a: &Option<Surface>, b: &Option<Surface>) -> bool {
    let bend = |s: &Option<Surface>| match s {
        Some(Surface::Cylinder { radius, .. }) | Some(Surface::Sphere { radius, .. }) => Some(*radius),
        Some(Surface::Torus { minor, .. }) => Some(*minor),
        _ => None,
    };
    match (bend(a), bend(b)) {
        (Some(x), Some(y)) => x.max(y) <= 1.25 * x.min(y),
        _ => true,
    }
}

fn kind_of(s: &Surface) -> Kind {
    match s {
        Surface::Plane { .. } => Kind::Plane,
        Surface::Cylinder { .. } => Kind::Cylinder,
        Surface::Cone { .. } => Kind::Cone,
        Surface::Sphere { .. } => Kind::Sphere,
        Surface::Torus { .. } => Kind::Torus,
        // a free form and a helix are never refitted by kind; they stand where a plane would be asked for nothing
        Surface::Free { .. } | Surface::Spline(_) | Surface::Helix { .. } | Surface::RoundHelix { .. } | Surface::Coil { .. } => Kind::Plane,
    }
}

/// What every fit of a set of triangles reads, gathered once: the distinct corners, the triangles' normals, and - only
/// when a cone or a torus asks - each corner with its normal averaged round it. A join tries up to four kinds on the
/// same union, and each gathering its own cost a HashSet, a HashMap and their vectors every time.
struct Sample<'t> {
    tris: &'t [u32],
    pts: Vec<Vector3<f64>>,
    normals: Vec<Vector3<f64>>,
    corners: std::cell::OnceCell<CornerNormals>,
}

/// Each corner of a set of triangles with the normal averaged round it, by area.
struct CornerNormals {
    feet: Vec<Vector3<f64>>,
    normals: Vec<Vector3<f64>>,
}

impl<'t> Sample<'t> {
    fn of(g: &Grower, tris: &'t [u32]) -> Sample<'t> {
        let normals = tris.iter().filter(|&&t| !g.sliver[t as usize]).map(|&t| g.normal(t as usize)).collect();
        Sample { tris, pts: g.corners_of(tris), normals, corners: std::cell::OnceCell::new() }
    }

    fn corners(&self, g: &Grower) -> &CornerNormals {
        self.corners.get_or_init(|| g.corner_normals(self.tris))
    }
}

struct Grower<'a> {
    p: &'a Prepared,
    tol: &'a Tolerance,
    cos_sharp: f64,
    cos_angle: f64,
    sliver: &'a [bool],
}

impl Grower<'_> {
    fn corner(&self, t: usize, k: usize) -> Vector3<f64> {
        let v = self.p.mesh.verts[self.p.mesh.tris[t][k] as usize];
        Vector3::new(v.x, v.y, v.z)
    }

    fn centroid(&self, t: usize) -> Vector3<f64> {
        (self.corner(t, 0) + self.corner(t, 1) + self.corner(t, 2)) / 3.0
    }

    fn normal(&self, t: usize) -> Vector3<f64> {
        let n = self.p.normals[t];
        Vector3::new(n[0], n[1], n[2])
    }

    fn area(&self, t: usize) -> f64 {
        (self.corner(t, 1) - self.corner(t, 0)).cross(&(self.corner(t, 2) - self.corner(t, 0))).norm() * 0.5
    }

    /// Whether the edge between `t` and `u` is sharper than the threshold. A sliver's normal means nothing, so an
    /// edge of a sliver is never taken for sharp.
    fn sharp(&self, t: usize, u: usize) -> bool {
        !self.sliver[t] && !self.sliver[u] && self.normal(t).dot(&self.normal(u)) < self.cos_sharp
    }

    fn corners_on(&self, t: usize, s: &Surface) -> bool {
        (0..3).all(|k| distance(s, &self.corner(t, k)) <= self.tol.distance)
    }

    /// Whether triangle `t` lies on `s`: its corners within the distance, its normal within the angle of the
    /// surface's normal (either way round: a hole's wall faces its axis).
    ///
    /// THE NORMAL IS COMPARED AT THE CENTRE OF THE CIRCLE THROUGH THE CORNERS, not at the centroid. A triangle with
    /// its corners on a sphere has a normal through the sphere's centre only at that point; at the centroid of a
    /// skewed one they part by degrees, and the corner spheres of the rounded bar came apart into single triangles.
    /// On a cylinder's strip that centre falls midway between the two lines, where the normals agree anyway.
    /// Whether triangle `t` is wide enough for its normal to mean something: corners moved within the distance tolerance
    /// turn it by up to atan(2 x tolerance / h), h its least height, and that stays within the angle tolerance.
    fn normal_trusted(&self, t: usize) -> bool {
        let (a, b, c) = (self.corner(t, 0), self.corner(t, 1), self.corner(t, 2));
        let longest = (b - a).norm().max((c - b).norm()).max((a - c).norm());
        let least_height = (b - a).cross(&(c - a)).norm() / longest.max(1e-300);
        least_height * self.tol.angle_deg.to_radians().tan() >= 2.0 * self.tol.distance
    }

    fn fits(&self, t: usize, s: &Surface) -> bool {
        // ...AND AT THE CENTROID, for a long blunt triangle: the centre of its circle stands far aside, and a cylinder's
        // normal there is another's - measured on the wall under a plane cutting a cylinder of 10 at 30 deg, strips from
        // the foot up to the cut were 10.6 deg off at that centre and 0.6 at the centroid, and the wall lay on no cylinder
        let agrees = |q: &Vector3<f64>| normal_at(s, q).is_some_and(|n| n.dot(&self.normal(t)).abs() >= self.cos_angle);
        self.corners_on(t, s) && (self.sliver[t] || agrees(&self.circumcentre(t)) || agrees(&self.centroid(t)))
    }

    fn circumcentre(&self, t: usize) -> Vector3<f64> {
        let (a, b, c) = (self.corner(t, 0), self.corner(t, 1), self.corner(t, 2));
        let (ab, ac) = (b - a, c - a);
        let n = ab.cross(&ac);
        let nn = n.norm_squared();
        if nn < 1e-30 {
            return self.centroid(t);
        }
        a + (n.cross(&ab) * ac.norm_squared() + ac.cross(&n) * ab.norm_squared()) / (2.0 * nn)
    }

    /// The seed and its neighbours `rings` out that belong to no region yet and are not across a sharp edge.
    fn patch(&self, seed: usize, rings: usize, owner: &[u32]) -> Vec<u32> {
        let mut out = vec![seed as u32];
        let mut ring = vec![seed];
        for _ in 0..rings {
            let mut next = Vec::new();
            for &t in &ring {
                for &u in &self.p.neighbours[t] {
                    if u == NO_NEIGHBOUR || owner[u as usize] != NONE || self.sliver[u as usize] || out.contains(&u) || self.sharp(t, u as usize) {
                        continue;
                    }
                    out.push(u);
                    next.push(u as usize);
                }
            }
            ring = next;
        }
        out
    }

    /// The surfaces of `kind` a seed may lie on, each to be grown and the widest kept.
    ///
    /// ONE GUESS FROM A PATCH AROUND THE SEED IS NOT ENOUGH. At the end of a rounding's strip the patch two rings
    /// out reaches into the corner sphere, the axis taken from those mixed normals is askew, and the cylinder died at
    /// its first step: measured on the rounded bar, the roundings came out as 4-triangle "planes". The axis of a
    /// cylinder is square to both normals of any two of its triangles, so every neighbour of the seed gives an axis
    /// of its own - the one across the strip gives the true one - and each is tried.
    fn guesses(&self, seed: usize, kind: Kind, owner: &[u32]) -> Vec<Surface> {
        let mut out = Vec::new();
        match kind {
            Kind::Plane => {
                let (c, n) = (self.centroid(seed), self.normal(seed));
                out.push(Surface::Plane { point: c.into(), normal: n.into() });
            }
            Kind::Cylinder => {
                for &u in &self.p.neighbours[seed] {
                    if u == NO_NEIGHBOUR || owner[u as usize] != NONE || self.sliver[u as usize] || self.sharp(seed, u as usize) {
                        continue;
                    }
                    let Some(axis) = self.normal(seed).cross(&self.normal(u as usize)).try_normalize(1e-9) else { continue };
                    out.extend(circle_about(&self.corners_of(&[seed as u32, u]), &axis));
                }
                out.extend(self.fit(&self.patch(seed, 2, owner), kind));
                // AND FROM THE SMOOTH PART OF THAT PATCH ALONE, the triangles turned from the seed by less than the
                // angle tolerance. Measured on a cylinder of 10 under a sphere set 0.1 mm off its axis: the patch two
                // rings out took in three triangles of the sphere, turned 13 to 17 deg - less than a sharp edge - and
                // the axis from their normals lay across the wall instead of along it.
                let smooth: Vec<u32> = self.patch(seed, 2, owner).into_iter().filter(|&t| self.normal(t as usize).dot(&self.normal(seed)) >= self.cos_angle).collect();
                out.extend(self.fit(&smooth, kind));
            }
            Kind::Sphere => {
                out.extend(self.fit(&self.patch(seed, 1, owner), kind));
                out.extend(self.fit(&self.patch(seed, 2, owner), kind));
            }
            // a cone and a torus need their normals to spread along the axis too, so the patch reaches further
            Kind::Cone | Kind::Torus => {
                out.extend(self.fit(&self.patch(seed, 2, owner), kind));
                out.extend(self.fit(&self.patch(seed, 3, owner), kind));
            }
        }
        out
    }

    /// The surface of every kind tried at `seed`, each grown over the free neighbours lying on it, and the one that
    /// covers the most area kept; `None` where none holds the seed. `area` is the area of every triangle.
    fn widest_at(&self, seed: usize, owner: &[u32], area: &[f64]) -> Option<Grown> {
        let mut best: Option<Grown> = None;
        for kind in [Kind::Plane, Kind::Cylinder, Kind::Cone, Kind::Sphere] {
            let mut widest: Option<Grown> = None;
            for s in self.guesses(seed, kind, owner) {
                let Some((tris, s)) = self.grow(seed, s, kind, owner) else { continue };
                let covered: f64 = tris.iter().map(|&t| area[t as usize]).sum();
                if widest.as_ref().is_none_or(|w| covered > w.area * (1.0 + 1e-9)) {
                    widest = Some(Grown { area: covered, tris, surface: s });
                }
            }
            // A MORE INVOLVED SURFACE HAS TO EARN ITS PLACE. Corners are all that is checked, and a plane face of
            // two triangles has only four: measured on the rounded bar, a 114 mm cylinder through them and the two
            // tangent strips beside them covered 3% more than the face itself and took it. A rounding's cylinder
            // covers eleven times the plane of one of its strips.
            if let Some(w) = widest {
                if best.as_ref().is_none_or(|b| w.area > b.area * SIMPLER_WINS) {
                    best = Some(w);
                }
            }
        }
        best
    }

    /// Every corner of `tris` with its normal: the area-weighted mean of the normals of the triangles of `tris` round it.
    fn corner_normals(&self, tris: &[u32]) -> CornerNormals {
        let mut sum: std::collections::HashMap<u32, Vector3<f64>> = std::collections::HashMap::new();
        for &t in tris {
            let t = t as usize;
            if self.sliver[t] {
                continue;
            }
            let weighted = self.normal(t) * self.area(t);
            for &i in &self.p.mesh.tris[t] {
                *sum.entry(i).or_insert_with(Vector3::zeros) += weighted;
            }
        }
        let mut keys: Vec<u32> = sum.keys().copied().collect();
        keys.sort_unstable();
        let (mut feet, mut normals) = (Vec::with_capacity(keys.len()), Vec::with_capacity(keys.len()));
        for i in keys {
            if let Some(n) = sum[&i].try_normalize(1e-12) {
                let v = self.p.mesh.verts[i as usize];
                feet.push(Vector3::new(v.x, v.y, v.z));
                normals.push(n);
            }
        }
        CornerNormals { feet, normals }
    }

    /// The distinct corners of `tris`.
    fn corners_of(&self, tris: &[u32]) -> Vec<Vector3<f64>> {
        let mut seen = HashSet::new();
        let mut pts = Vec::new();
        for &t in tris {
            for &i in &self.p.mesh.tris[t as usize] {
                if seen.insert(i) {
                    let v = self.p.mesh.verts[i as usize];
                    pts.push(Vector3::new(v.x, v.y, v.z));
                }
            }
        }
        pts
    }

    /// The surface of `kind` fitted to the corners of `tris` by least squares; a cylinder takes its axis from the
    /// normals, which a tessellation gets right at every triangle.
    fn fit(&self, tris: &[u32], kind: Kind) -> Option<Surface> {
        self.fit_sample(&Sample::of(self, tris), kind)
    }

    fn fit_sample(&self, sample: &Sample, kind: Kind) -> Option<Surface> {
        let accept = SHORT_REFINE_TO * self.tol.distance;
        match kind {
            Kind::Plane => fit_plane(&sample.pts),
            Kind::Cylinder => fit_cylinder(&sample.pts, &sample.normals, accept),
            Kind::Cone => {
                let CornerNormals { feet, normals } = sample.corners(self);
                fit_cone(&sample.pts, feet, normals, accept)
            }
            Kind::Sphere => fit_sphere(&sample.pts),
            Kind::Torus => {
                let CornerNormals { feet, normals } = sample.corners(self);
                fit_torus(&sample.pts, feet, normals, accept)
            }
        }
    }

    /// Grow from `seed` over every neighbour that lies on the surface, refitting it to the whole region each time
    /// the region doubles. `None` when the seed itself does not lie on it.
    fn grow(&self, seed: usize, mut s: Surface, kind: Kind, owner: &[u32]) -> Option<(Vec<u32>, Surface)> {
        if !self.fits(seed, &s) {
            return None;
        }
        let mut inside = vec![seed as u32];
        let mut member: HashSet<u32> = HashSet::from([seed as u32]);
        let mut tried: HashSet<u32> = HashSet::from([seed as u32]);
        let mut queue = VecDeque::from([seed]);
        let mut refit_at = 8;
        // GROWN IN ROUNDS: after each the surface is refitted to the whole region, and every neighbour turned down gets
        // another look - it was measured against the surface of the few triangles the region then had. A short arc of a
        // few noisy corners puts a cylinder's axis off: on a cylinder of 10 with its corners moved by 0.02, a guess stood
        // 0.18 mm off its axis, stopped at 11 triangles of the wall's 90, and a sphere through both rims took the wall.
        for _round in 0..4 {
            let before = inside.len();
            while let Some(t) = queue.pop_front() {
                if !member.contains(&(t as u32)) {
                    continue; // left the region at a refit
                }
                for &u in &self.p.neighbours[t] {
                    if u == NO_NEIGHBOUR || owner[u as usize] != NONE || tried.contains(&u) {
                        continue;
                    }
                    let ui = u as usize;
                    if self.sharp(t, ui) || !self.fits(ui, &s) {
                        continue;
                    }
                    tried.insert(u);
                    member.insert(u);
                    inside.push(u);
                    queue.push_back(ui);
                    if inside.len() >= refit_at {
                        s = self.refit(&mut inside, &mut member, seed, kind).unwrap_or(s);
                        refit_at *= 2;
                    }
                }
            }
            s = self.refit(&mut inside, &mut member, seed, kind).unwrap_or(s);
            if inside.len() <= before {
                break;
            }
            queue = inside.iter().map(|&t| t as usize).collect();
        }
        Some((inside, s))
    }

    /// The surface refitted to the whole region. A triangle taken early, on a rougher guess, may not lie on the
    /// better surface: measured on the rounded bar, a rounding took one triangle of each corner sphere, its radius
    /// came out 2.0025 instead of 2, and a refit that every member had to fit was never taken. So the members off
    /// the refit leave - if they are few and the seed is not among them - and the rest is fitted once more.
    fn refit(&self, inside: &mut Vec<u32>, member: &mut HashSet<u32>, seed: usize, kind: Kind) -> Option<Surface> {
        let f = self.fit(inside, kind)?;
        let (stay, leave): (Vec<u32>, Vec<u32>) = inside.iter().partition(|&&t| self.fits(t as usize, &f));
        if leave.is_empty() {
            return Some(f);
        }
        if leave.len() * MAY_LEAVE > inside.len() || leave.contains(&(seed as u32)) {
            return None;
        }
        let g = self.fit(&stay, kind).filter(|g| stay.iter().all(|&t| self.fits(t as usize, g)))?;
        for t in &leave {
            member.remove(t);
        }
        *inside = stay;
        Some(g)
    }
}

fn distance(s: &Surface, v: &Vector3<f64>) -> f64 {
    match s {
        Surface::Plane { point, normal } => (v - Vector3::from(*point)).dot(&Vector3::from(*normal)).abs(),
        Surface::Cylinder { point, axis, radius } => {
            let d = v - Vector3::from(*point);
            let a = Vector3::from(*axis);
            ((d - a * d.dot(&a)).norm() - radius).abs()
        }
        Surface::Sphere { center, radius } => ((v - Vector3::from(*center)).norm() - radius).abs(),
        Surface::Cone { apex, axis, half_angle } => {
            let d = v - Vector3::from(*apex);
            let a = Vector3::from(*axis);
            let z = d.dot(&a);
            ((d - a * z).norm() * half_angle.cos() - z * half_angle.sin()).abs()
        }
        Surface::Torus { center, axis, major, minor } => {
            let d = v - Vector3::from(*center);
            let a = Vector3::from(*axis);
            let z = d.dot(&a);
            (((d - a * z).norm() - major).hypot(z) - minor).abs()
        }
        // nothing lies on a free form by being near it: it is whatever its own triangles are
        Surface::Free { .. } | Surface::Spline(_) => f64::INFINITY,
        Surface::Helix { .. } | Surface::RoundHelix { .. } => helix_at(s, v).map_or(f64::INFINITY, |h| h.off.abs()),
        Surface::Coil { .. } => coil_at(s, v).map_or(f64::INFINITY, |c| c.off.abs()),
    }
}

/// The unit normal of `s` at the point nearest `v`, or `None` on a cylinder's axis or a sphere's centre.
fn normal_at(s: &Surface, v: &Vector3<f64>) -> Option<Vector3<f64>> {
    let n = match s {
        Surface::Plane { normal, .. } => Vector3::from(*normal),
        Surface::Cylinder { point, axis, .. } => {
            let d = v - Vector3::from(*point);
            let a = Vector3::from(*axis);
            d - a * d.dot(&a)
        }
        Surface::Sphere { center, .. } => v - Vector3::from(*center),
        Surface::Cone { apex, axis, half_angle } => {
            let d = v - Vector3::from(*apex);
            let a = Vector3::from(*axis);
            let out = (d - a * d.dot(&a)).try_normalize(1e-12)?;
            out * half_angle.cos() - a * half_angle.sin()
        }
        Surface::Torus { center, axis, major, .. } => {
            let d = v - Vector3::from(*center);
            let a = Vector3::from(*axis);
            let out = (d - a * d.dot(&a)).try_normalize(1e-12)?;
            d - out * *major
        }
        Surface::Free { .. } | Surface::Spline(_) => return None,
        Surface::Helix { .. } | Surface::RoundHelix { .. } => helix_at(s, v)?.normal,
        Surface::Coil { .. } => coil_at(s, v)?.out,
    };
    n.try_normalize(1e-12)
}

fn mean(pts: &[Vector3<f64>]) -> Vector3<f64> {
    pts.iter().fold(Vector3::zeros(), |a, p| a + p) / pts.len().max(1) as f64
}

/// The eigenvectors of a symmetric matrix, smallest eigenvalue first, with the eigenvalues.
fn eigen_sorted(m: Matrix3<f64>) -> ([f64; 3], [Vector3<f64>; 3]) {
    let e = SymmetricEigen::new(m);
    let mut idx = [0, 1, 2];
    idx.sort_by(|&a, &b| e.eigenvalues[a].total_cmp(&e.eigenvalues[b]));
    (idx.map(|i| e.eigenvalues[i]), idx.map(|i| e.eigenvectors.column(i).into_owned()))
}

fn fit_plane(pts: &[Vector3<f64>]) -> Option<Surface> {
    if pts.len() < 3 {
        return None;
    }
    let c = mean(pts);
    let m = pts.iter().fold(Matrix3::zeros(), |a, p| a + (p - c) * (p - c).transpose());
    let (_, v) = eigen_sorted(m);
    Some(Surface::Plane { point: c.into(), normal: v[0].normalize().into() })
}

/// The axis is the direction every normal of a cylinder is square to: the least eigenvector of the normals'
/// scatter. Normals that all point one way (a flat patch) give no axis. The circle is then fitted to the corners
/// dropped onto the plane across the axis (the algebraic fit: linear, and exact for points on a circle).
///
/// THEN THE AXIS IS REFINED ON THE CORNERS. A triangle's normal is square to the axis only when the triangle stands
/// square on the wall; a long strip whose top corner sits at another angle than its foot - the wall under a curve two
/// surfaces meet along - tilts its normal, and over a strip 27 mm tall that tilt moves the corners dropped across the
/// axis as far as the sag of the arc itself. Measured on a cylinder of 10 under a sphere set 0.1 mm off its axis: the
/// corners lay on the wall exactly, and the wall still came in 70 cylinders of 12.1 to 150 over 105 regions.
fn fit_cylinder(pts: &[Vector3<f64>], normals: &[Vector3<f64>], accept: f64) -> Option<Surface> {
    if pts.len() < 3 || normals.len() < 2 {
        return None;
    }
    let m = normals.iter().fold(Matrix3::zeros(), |a, n| a + n * n.transpose());
    let (val, vec) = eigen_sorted(m);
    if val[1] < 1e-9 * val[2] {
        return None; // the normals span no plane: a flat patch
    }
    let first = circle_about(pts, &vec[0].normalize())?;
    // five numbers: the axis tilted along two directions across it, its foot moved along the same two, the radius
    if pts.len() < 6 {
        return Some(first);
    }
    let Surface::Cylinder { point, axis, radius } = first else { return Some(first) };
    let (c0, base) = (Vector3::from(point), Vector3::from(axis));
    let (u, w) = across(&base);
    let turn = Turn { base, u, w };
    let miss = |q: &[f64; 5], v: &Vector3<f64>| cylinder_miss(&turn, &c0, q, v);
    let mut q = [0.0, 0.0, 0.0, 0.0, radius];
    if rms(&q, pts, &miss) <= accept || !refined(&mut q, pts, &miss, accept) || q[4].is_nan() || q[4] <= 0.0 {
        return Some(first);
    }
    let a = (base + u * q[0] + w * q[1]).normalize();
    let c = c0 + u * q[2] + w * q[3];
    Some(Surface::Cylinder { point: c.into(), axis: a.into(), radius: q[4] })
}

/// How far `v` lies off the cylinder `q` - the axis `turn.base` tilted by `q[0]` and `q[1]` along `turn.u` and `turn.w`,
/// its foot `c0` moved by `q[2]` and `q[3]` along them, radius `q[4]` - with the derivatives by those five numbers.
fn cylinder_miss(turn: &Turn, c0: &Vector3<f64>, q: &[f64; 5], v: &Vector3<f64>) -> (f64, [f64; 5]) {
    let (u, w) = (turn.u, turn.w);
    let big = turn.base + u * q[0] + w * q[1];
    let len = big.norm();
    let a = big / len;
    let d = v - (c0 + u * q[2] + w * q[3]);
    let along = d.dot(&a);
    let e = d - a * along;
    let dist = e.norm();
    if dist < 1e-12 {
        return (-q[4], [0.0, 0.0, 0.0, 0.0, -1.0]);
    }
    let h = e / dist;
    // the axis turns by q[0] and q[1] as da = (u - a (a.u)) / len; the distance moves against the along-axis part of that
    let da0 = (u - a * a.dot(&u)) / len;
    let da1 = (w - a * a.dot(&w)) / len;
    let dd0 = -(h.dot(&da0) * along + h.dot(&a) * d.dot(&da0));
    let dd1 = -(h.dot(&da1) * along + h.dot(&a) * d.dot(&da1));
    let dc0 = -(h.dot(&u) - h.dot(&a) * a.dot(&u));
    let dc1 = -(h.dot(&w) - h.dot(&a) * a.dot(&w));
    (dist - q[4], [dd0, dd1, dc0, dc1, -1.0])
}

/// The cylinder about a given axis direction: the circle fitted to the corners dropped onto the plane across it.
fn circle_about(pts: &[Vector3<f64>], axis: &Vector3<f64>) -> Option<Surface> {
    if pts.len() < 3 {
        return None;
    }
    let axis = *axis;
    let u = axis.cross(&if axis.x.abs() < 0.9 { Vector3::x() } else { Vector3::y() }).normalize();
    let w = axis.cross(&u);
    let c = mean(pts);
    let flat: Vec<(f64, f64)> = pts.iter().map(|p| ((p - c).dot(&u), (p - c).dot(&w))).collect();
    let (mut ata, mut atb) = (Matrix3::zeros(), Vector3::zeros());
    for &(x, y) in &flat {
        let row = Vector3::new(x, y, 1.0);
        ata += row * row.transpose();
        atb += row * -(x * x + y * y);
    }
    let sol = ata.lu().solve(&atb)?;
    let (cx, cy) = (-sol[0] / 2.0, -sol[1] / 2.0);
    let r2 = cx * cx + cy * cy - sol[2];
    if r2 <= 0.0 || !r2.is_finite() {
        return None;
    }
    let point = c + u * cx + w * cy;
    Some(Surface::Cylinder { point: point.into(), axis: axis.into(), radius: r2.sqrt() })
}

/// The algebraic sphere through the corners: linear in the centre and in r^2 - |c|^2. Corners on one plane give no
/// sphere.
fn fit_sphere(pts: &[Vector3<f64>]) -> Option<Surface> {
    if pts.len() < 4 {
        return None;
    }
    let c = mean(pts);
    let (mut ata, mut atb) = (Matrix4::zeros(), Vector4::zeros());
    for p in pts {
        let d = p - c;
        let row = Vector4::new(d.x, d.y, d.z, 1.0);
        ata += row * row.transpose();
        atb += row * -d.norm_squared();
    }
    let sol = ata.lu().solve(&atb)?;
    let o = Vector3::new(-sol[0] / 2.0, -sol[1] / 2.0, -sol[2] / 2.0);
    let r2 = o.norm_squared() - sol[3];
    if r2 <= 0.0 || !r2.is_finite() {
        return None;
    }
    Some(Surface::Sphere { center: (c + o).into(), radius: r2.sqrt() })
}

/// The axis of a surface of revolution: the one line every normal's line meets - on a cylinder, a cone and a torus
/// alike. In Plucker coordinates a line (a, M) meets the line through p along n when a . (p x n) + M . n = 0, which
/// is linear in the six numbers, so the axis is the least eigenvector of a 6 x 6 matrix. It replaced a search along
/// the tube's centre points, which on a narrow band of a torus lie on too short an arc to fit a circle to.
/// Returns the unit direction and the point of the axis nearest the feet's middle; `None` when the normals do not
/// pin a line down.
fn axis_by_normal_lines(feet: &[Vector3<f64>], normals: &[Vector3<f64>]) -> Option<(Vector3<f64>, Vector3<f64>)> {
    if normals.len() < 5 {
        return None;
    }
    let o = mean(feet);
    let mut m = nalgebra::Matrix6::<f64>::zeros();
    for (p, n) in feet.iter().zip(normals) {
        let row = nalgebra::Vector6::from_iterator((p - o).cross(n).iter().chain(n.iter()).copied());
        m += row * row.transpose();
    }
    let e = SymmetricEigen::new(m);
    let k = (0..6).min_by(|&a, &b| e.eigenvalues[a].total_cmp(&e.eigenvalues[b]))?;
    let x = e.eigenvectors.column(k);
    let (a, big_m) = (Vector3::new(x[0], x[1], x[2]), Vector3::new(x[3], x[4], x[5]));
    let len = a.norm();
    if len < 1e-9 {
        return None;
    }
    let (a, big_m) = (a / len, big_m / len);
    Some((a, o + a.cross(&big_m)))
}

/// Two unit vectors square to `a` and to each other.
fn across(a: &Vector3<f64>) -> (Vector3<f64>, Vector3<f64>) {
    let u = a.cross(&if a.x.abs() < 0.9 { Vector3::x() } else { Vector3::y() }).normalize();
    (u, a.cross(&u))
}

/// Each corner as (height along the axis, distance from it).
fn profile(pts: &[Vector3<f64>], a: &Vector3<f64>, on: &Vector3<f64>) -> Vec<(f64, f64)> {
    pts.iter()
        .map(|p| {
            let d = p - on;
            let z = d.dot(a);
            (z, (d - a * z).norm())
        })
        .collect()
}

/// The size of what the corners span, for judging a first fit before refining it.
fn span(pts: &[Vector3<f64>]) -> f64 {
    let (lo, hi) = pts.iter().fold((Vector3::repeat(f64::MAX), Vector3::repeat(f64::MIN)), |(lo, hi), p| (lo.inf(p), hi.sup(p)));
    (hi - lo).norm()
}

/// The root mean square of `residual` over the corners.
fn rms<const N: usize>(q: &[f64; N], pts: &[Vector3<f64>], miss: &impl Fn(&[f64; N], &Vector3<f64>) -> (f64, [f64; N])) -> f64 {
    (pts.iter().map(|p| miss(q, p).0.powi(2)).sum::<f64>() / pts.len().max(1) as f64).sqrt()
}

/// A first fit off by more than this share of what it spans is no fit to refine: it is some other surface. The first
/// fit of a torus band is off by 5 - 8% of its span, so the share is wide; the early stop of the refine does the rest.
const NOT_EVEN_CLOSE: f64 = 0.1;

/// A cone: in the axis's frame the corners lie on a straight line of distance against height, which gives the
/// half-angle and the apex. Refined on the corners, which lie on the surface.
fn fit_cone(pts: &[Vector3<f64>], feet: &[Vector3<f64>], normals: &[Vector3<f64>], accept: f64) -> Option<Surface> {
    let (a, on) = axis_by_normal_lines(feet, normals)?;
    let zr = profile(pts, &a, &on);
    let n = zr.len() as f64;
    let (mz, mr) = (zr.iter().map(|x| x.0).sum::<f64>() / n, zr.iter().map(|x| x.1).sum::<f64>() / n);
    let var = zr.iter().map(|x| (x.0 - mz).powi(2)).sum::<f64>();
    if var < 1e-12 {
        return None; // no length along the axis to tell a slope from
    }
    let k = zr.iter().map(|x| (x.0 - mz) * (x.1 - mr)).sum::<f64>() / var;
    if k.abs() < 1e-3 {
        return None; // a cylinder
    }
    let apex = on + a * (mz - mr / k);
    let axis = if k > 0.0 { a } else { -a };
    let (u, w) = across(&axis);
    let mut q = [apex.x, apex.y, apex.z, 0.0, 0.0, k.abs().atan()];
    let turn = Turn { base: axis, u, w };
    let at = |q: &[f64; 6]| (Vector3::new(q[0], q[1], q[2]), turn.tilted(q[3], q[4]).0, q[5]);
    let miss = |q: &[f64; 6], v: &Vector3<f64>| cone_miss(&turn, q, v);
    if rms(&q, pts, &miss) > NOT_EVEN_CLOSE * span(pts) {
        return None;
    }
    if !refined(&mut q, pts, &miss, accept) {
        return None;
    }
    let (apex, axis, half) = at(&q);
    (half > 1e-4 && half < std::f64::consts::FRAC_PI_2 - 1e-4).then(|| Surface::Cone { apex: apex.into(), axis: axis.into(), half_angle: half })
}

/// A torus: in the axis's frame the corners lie on a circle of distance against height - the tube's section - which
/// gives both radii and the centre. Refined on the corners.
fn fit_torus(pts: &[Vector3<f64>], feet: &[Vector3<f64>], normals: &[Vector3<f64>], accept: f64) -> Option<Surface> {
    let (a, on) = axis_by_normal_lines(feet, normals)?;
    let zr = profile(pts, &a, &on);
    let n = zr.len() as f64;
    let (mz, mr) = (zr.iter().map(|x| x.0).sum::<f64>() / n, zr.iter().map(|x| x.1).sum::<f64>() / n);
    let (mut lhs, mut rhs) = (Matrix3::zeros(), Vector3::zeros());
    for &(z, r) in &zr {
        let (x, y) = (r - mr, z - mz);
        let row = Vector3::new(x, y, 1.0);
        lhs += row * row.transpose();
        rhs += row * -(x * x + y * y);
    }
    let sol = lhs.lu().solve(&rhs)?;
    let (cx, cy) = (-sol[0] / 2.0, -sol[1] / 2.0);
    let r2 = cx * cx + cy * cy - sol[2];
    let (major, z0) = (mr + cx, mz + cy);
    if r2.is_nan() || r2 <= 0.0 || major.is_nan() || major <= 0.0 {
        return None;
    }
    let centre = on + a * z0;
    let (u, w) = across(&a);
    let mut q = [centre.x, centre.y, centre.z, 0.0, 0.0, major, r2.sqrt()];
    let turn = Turn { base: a, u, w };
    let at = |q: &[f64; 7]| (Vector3::new(q[0], q[1], q[2]), turn.tilted(q[3], q[4]).0, q[5], q[6]);
    let miss = |q: &[f64; 7], v: &Vector3<f64>| torus_miss(&turn, q, v);
    if rms(&q, pts, &miss) > NOT_EVEN_CLOSE * span(pts) {
        return None;
    }
    if !refined(&mut q, pts, &miss, accept) {
        return None;
    }
    let (centre, axis, major, minor) = at(&q);
    (major > 0.0 && minor > 0.0 && major.is_finite() && minor.is_finite()).then(|| Surface::Torus { center: centre.into(), axis: axis.into(), major, minor })
}

/// The frame an axis is turned in while a cone or a torus is refined: the axis the fit started from and two unit
/// vectors across it. The axis is `base + s u + t w`, made unit.
struct Turn {
    base: Vector3<f64>,
    u: Vector3<f64>,
    w: Vector3<f64>,
}

impl Turn {
    /// The axis at (s, t), and how it moves with s and with t.
    fn tilted(&self, s: f64, t: f64) -> (Vector3<f64>, Vector3<f64>, Vector3<f64>) {
        let m = self.base + self.u * s + self.w * t;
        let len = m.norm();
        let a = m / len;
        (a, (self.u - a * a.dot(&self.u)) / len, (self.w - a * a.dot(&self.w)) / len)
    }
}

/// How far `v` lies off the cone q = (apex, s, t, half-angle), signed, with its derivatives in q. In the axis's frame a
/// corner is at height z and distance rho; the miss is rho cos - z sin, and moving the apex moves the corner the other
/// way, turning the axis moves z by d and rho by -z along the corner's own direction across it.
fn cone_miss(turn: &Turn, q: &[f64; 6], v: &Vector3<f64>) -> (f64, [f64; 6]) {
    let (a, by_s, by_t) = turn.tilted(q[3], q[4]);
    let d = v - Vector3::new(q[0], q[1], q[2]);
    let z = d.dot(&a);
    let e = d - a * z;
    let rho = e.norm();
    let out = if rho > 1e-15 { e / rho } else { Vector3::zeros() };
    let (sin, cos) = q[5].sin_cos();
    let by_d = out * cos - a * sin;
    let by_axis = -(out * (z * cos)) - d * sin;
    (rho * cos - z * sin, [-by_d.x, -by_d.y, -by_d.z, by_axis.dot(&by_s), by_axis.dot(&by_t), -rho * sin - z * cos])
}

/// How far `v` lies off the torus q = (centre, s, t, major, minor), signed, with its derivatives in q: the distance
/// from the tube's centre circle less the tube's radius, moved by the centre against the surface's normal.
fn torus_miss(turn: &Turn, q: &[f64; 7], v: &Vector3<f64>) -> (f64, [f64; 7]) {
    let (a, by_s, by_t) = turn.tilted(q[3], q[4]);
    let d = v - Vector3::new(q[0], q[1], q[2]);
    let z = d.dot(&a);
    let e = d - a * z;
    let rho = e.norm();
    let out = if rho > 1e-15 { e / rho } else { Vector3::zeros() };
    let off = rho - q[5];
    let from_circle = off.hypot(z).max(1e-300);
    let by_d = out * (off / from_circle) + a * (z / from_circle);
    let by_axis = (d - out * off) * (z / from_circle);
    (from_circle - q[6], [-by_d.x, -by_d.y, -by_d.z, by_axis.dot(&by_s), by_axis.dot(&by_t), -off / from_circle, -1.0])
}

/// A first fit is refined on a few corners in one run of up to 23 steps, and taken - then refined in full - if that
/// brings it within this many tolerances. The run stops at the third step if the miss has not halved. Measured: a torus
/// from a print program's mesh went 0.33 -> 0.0096 in three steps and on to 4e-6 mm, a cone of our kernel to 1e-7 -
/// far inside one tolerance; on an organic mesh of 225 154 triangles a threshold of ten tolerances let 54 338 unions
/// through to the full refine, and 1 606 of 36 875 of them then lay on the surface - the rest took 10 s for nothing.
const SHORT_REFINE_TO: f64 = 1.0;

/// How far past the tolerance a single corner may lie after the short run for the full refine to be worth it.
const WORST_OFF: f64 = 1.0;

/// Refine `q` in one run on at most 60 corners - enough to tell a torus band of some sixty from an organic patch -
/// dropping it early if it is not closing in; whether it came within `accept`.
fn refined<const N: usize>(q: &mut [f64; N], pts: &[Vector3<f64>], miss: &impl Fn(&[f64; N], &Vector3<f64>) -> (f64, [f64; N]), accept: f64) -> bool {
    let first = rms(q, pts, miss);
    let now = refine(q, pts, miss, 23, 60, Some((3, first / 2.0)), accept);
    if now > accept {
        return false;
    }
    // THE WORST CORNER, NOT THE MEAN: two dozen corners of an organic patch take a torus of seven numbers within the
    // tolerance on the mean and still leave a corner out, so the union never lies on it. Measured: 25 114 such fits,
    // 1 576 of them lying on the surface; a fit of a CAD mesh is off by 1e-6 at its worst corner after the short run.
    if pts.iter().any(|p| miss(q, p).0.abs() > WORST_OFF * accept) {
        return false;
    }
    refine(q, pts, miss, 20, 1500, None, accept / (SHORT_REFINE_TO * 10.0));
    true
}

/// Least squares on the corners' distances to a surface given by `q`: Gauss-Newton with a Marquardt damping, the
/// derivatives by small steps. The corners of a CAD mesh lie on the surface, so the minimum is the exact surface;
/// a first guess from the triangles' middles is off by the tessellation's own sag. At most `most` corners are
/// used, evenly picked, for at most `steps` steps, stopping once the root mean square miss is within `enough`.
fn refine<const N: usize>(
    q: &mut [f64; N],
    pts: &[Vector3<f64>],
    miss: &impl Fn(&[f64; N], &Vector3<f64>) -> (f64, [f64; N]),
    steps: usize,
    most: usize,
    give_up: Option<(usize, f64)>,
    enough: f64,
) -> f64 {
    let step = pts.len().div_ceil(most).max(1);
    let pts: Vec<&Vector3<f64>> = pts.iter().step_by(step).collect();
    let cost = |q: &[f64; N]| pts.iter().map(|p| miss(q, p).0.powi(2)).sum::<f64>();
    let mut now = cost(q);
    let mut damping = 1e-3;
    for step_no in 0..steps {
        let off_by = (now / pts.len().max(1) as f64).sqrt();
        // STOPPED AS SOON AS IT IS CLOSE ENOUGH. It ran every step to the end, and on an organic mesh of 225 154
        // triangles, where a small smooth patch takes a torus of seven numbers easily, the refines took 20 s of 23.
        if off_by <= enough || give_up.is_some_and(|(at, above)| step_no == at && off_by > above) {
            break;
        }
        // the normal equations are summed corner by corner, from derivatives written out by hand: a matrix as tall as
        // the corners was made at every step, and each derivative by a small step cost a whole miss more
        let (mut lhs, mut rhs) = (DMatrix::<f64>::zeros(N, N), DVector::<f64>::zeros(N));
        for p in &pts {
            let (r, grad) = miss(q, p);
            for a in 0..N {
                rhs[a] -= grad[a] * r;
                for b in 0..N {
                    lhs[(a, b)] += grad[a] * grad[b];
                }
            }
        }
        for k in 0..N {
            lhs[(k, k)] *= 1.0 + damping;
        }
        let Some(dq) = lhs.lu().solve(&rhs) else { break };
        let mut next = *q;
        for k in 0..N {
            next[k] += dq[k];
        }
        let then = cost(&next);
        if then < now {
            let gained = now - then;
            *q = next;
            now = then;
            damping *= 0.3;
            // a step that gains less than a ten-thousandth has found the bottom; where that bottom is above the
            // tolerance - an organic patch is no torus - every further step was paid for nothing
            if gained <= 1e-4 * now {
                break;
            }
        } else {
            damping *= 10.0;
            if damping > 1e12 {
                break;
            }
        }
    }
    (now / pts.len().max(1) as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE DERIVATIVES WRITTEN OUT BY HAND AGREE WITH SMALL STEPS OF THE MISS ITSELF. A wrong one would not fail a fit
    /// outright - the refine would still crawl towards the surface - so only this says it is right.
    #[test]
    fn the_written_out_derivatives_agree_with_small_steps() {
        let base = Vector3::new(0.2, -0.3, 0.93).normalize();
        let (u, w) = across(&base);
        let turn = Turn { base, u, w };
        let v = Vector3::new(3.0, -1.5, 7.0);
        agree("cone", &[0.5, 0.2, -1.0, 0.05, -0.03, 0.4], |q| cone_miss(&turn, q, &v));
        agree("torus", &[0.3, -0.2, 0.5, 0.04, 0.02, 6.0, 1.5], |q| torus_miss(&turn, q, &v));
        let foot = Vector3::new(0.4, 0.1, -0.2);
        agree("cylinder", &[0.05, -0.03, 0.2, -0.1, 4.0], |q| cylinder_miss(&turn, &foot, q, &v));
    }

    /// A CYLINDER IS REFINED ONTO CORNERS THAT LIE ON IT, from a start a little off - as the corners of a wall's strips
    /// under a slanted cut stand: an arc of 60 deg of a cylinder of 10, at heights 0 and 20.
    #[test]
    fn a_cylinder_is_refined_onto_its_corners() {
        let pts: Vec<Vector3<f64>> = (0..26)
            .map(|k| {
                let a = (60.0 + 60.0 * k as f64 / 25.0f64).to_radians();
                Vector3::new(10.0 * a.cos(), 10.0 * a.sin(), if k % 2 == 0 { 0.0 } else { 20.0 + 0.02 * k as f64 })
            })
            .collect();
        let normals: Vec<Vector3<f64>> = pts.iter().map(|p| Vector3::new(p.x, p.y, 0.0).normalize() + Vector3::new(0.0, 0.0, 2e-4)).collect();
        let fitted = fit_cylinder(&pts, &normals, 0.00035).expect("a cylinder");
        let worst = pts.iter().map(|p| distance(&fitted, p)).fold(0.0, f64::max);
        assert!(worst < 1e-5, "the cylinder {fitted:?} leaves a corner {worst} off");
    }

    fn agree<const N: usize>(name: &str, q: &[f64; N], miss: impl Fn(&[f64; N]) -> (f64, [f64; N])) {
        let (_, written) = miss(q);
        for k in 0..N {
            let h = 1e-6;
            let (mut up, mut down) = (*q, *q);
            up[k] += h;
            down[k] -= h;
            let stepped = (miss(&up).0 - miss(&down).0) / (2.0 * h);
            assert!((stepped - written[k]).abs() < 1e-5 * (1.0 + stepped.abs()), "{name}: derivative {k} written {}, by steps {stepped}", written[k]);
        }
    }
}
