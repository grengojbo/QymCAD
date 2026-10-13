//! WHAT A PERSON POINTS AT ON A BODY: a hole, a rounding or a chamfer, told from the place clicked. The surface there is
//! grown from the triangle under the pointer (`region_at`) over the body's own triangles, and what it is follows from
//! the surface and from the faces round it:
//!
//! * a cylinder going round its axis (more than `ROUND_HOLE`), its wall facing the axis, with a face square to the axis
//!   at an end that faces out - a hole; through where both ends open, blind where the other end is closed by faces
//!   lying within the bore;
//! * a cylinder going round less than `ROUND_BLEND`, met along its length by two faces - a rounding of the edge those
//!   two faces would meet in;
//! * a plane met along its length by two faces, each leaning 20 to 70 deg from it, the two meeting lines parallel and as
//!   far from the corner of those faces - a chamfer.
//!
//! The faces come from the body - a polyhedron made of a mesh, a body recognised into exact surfaces, a STEP - so the
//! same click works on all of them: the wall of a hole drawn as twelve flat strips lies on one cylinder as an exact wall
//! does, its corners on the circle. No interface lives here: the tool asks and shows what came back.

use crate::{prepare, region_at, weld_tolerance, Prepared, Surface, Tolerance, NO_NEIGHBOUR};
use nalgebra::{Matrix3, Vector3};
use qymcad_core::geom::{EdgeFaces, Mesh, MeshFace};
use std::collections::{HashMap, HashSet};

/// AN EDGE OF THE BODY IS NOT CROSSED ABOVE THIS, in degrees. The recognition of a whole mesh stops at 30 deg; here the
/// edge between two flat strips of a hole drawn with N of them turns 360 / N deg, and a hole of 8 strips (45 deg) has to
/// be one wall. The edge round the mouth of a hole turns 90 deg and still stops the wall; a chamfer's edges turn 45 and
/// do not stop it, but its plane holds no corner of the face beyond, and the region ends there all the same.
const SHARP_DEG: f64 = 50.0;

/// A wall going this far round its axis, in degrees, is a hole: a bore cut by a slot or a chamfer at its mouth leaves a
/// gap in the wall, and a rounding never goes past half a turn.
const ROUND_HOLE: f64 = 300.0;

/// A wall going round no further than this, in degrees, is a rounding: a quarter turn on a square edge, more on an
/// edge of faces meeting at an acute angle.
const ROUND_BLEND: f64 = 190.0;

/// How far round its axis one triangle of a round wall may turn, in degrees: a strip of a hole drawn with 8 of them
/// turns 45.
const STRIP_DEG: f64 = 46.0;

/// The place a person clicked: the face under the pointer, by its persistent id, and the point on it.
#[derive(Clone, Copy, Debug)]
pub struct Pick {
    pub face: u32,
    pub at: [f64; 3],
}

/// Whether a hole goes through the body or stops inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    Through,
    Blind,
}

/// A hole found on a body.
#[derive(Clone, Debug, PartialEq)]
pub struct Hole {
    /// The face the hole is drilled from, by its persistent id.
    pub entry: u32,
    /// Where the axis meets the entry face.
    pub at: [f64; 3],
    /// The unit vector along the axis from the entry into the body.
    pub inward: [f64; 3],
    pub diameter: f64,
    /// From the entry face to the far end of the wall.
    pub depth: f64,
    pub reach: Reach,
    /// The faces of the hole: its wall, and the bottom of a blind one.
    pub faces: Vec<u32>,
}

/// A rounding of an edge found on a body.
#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    pub radius: f64,
    /// The two faces whose edge was rounded.
    pub sides: EdgeFaces,
    /// The faces of the rounding.
    pub faces: Vec<u32>,
}

/// A chamfer of an edge found on a body, its two legs equal.
#[derive(Clone, Debug, PartialEq)]
pub struct Chamfer {
    /// How far the cut stands back from the edge along each face.
    pub leg: f64,
    /// The two faces whose edge was cut.
    pub sides: EdgeFaces,
    /// The faces of the chamfer.
    pub faces: Vec<u32>,
}

/// What was found at the place clicked.
#[derive(Clone, Debug, PartialEq)]
pub enum Found {
    Hole(Hole),
    Round(Round),
    Chamfer(Chamfer),
}

/// Why nothing was found: each is mended in its own way, and the status line says which.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unfound {
    /// No surface holds the triangle clicked within the tolerance: a rougher mesh needs a larger one.
    NoSurface,
    /// A round wall with no face to drill it from: a shaft, a boss.
    NoEntry,
    /// A part of a round wall met by no two faces along its length.
    NotTwoSides,
    /// A slanted face whose two legs differ: the chamfer is not even.
    Uneven,
    /// A surface that is none of a hole, a rounding or a chamfer.
    Other,
}

/// The hole, rounding or chamfer at `pick` on the body of `mesh`, whose triangles `faces` group into the body's faces.
/// `tol` multiplies the distance a corner may lie from the surface, as for a recognition: 1 for a body built by a
/// kernel, more for a rough mesh.
pub fn feature_at(mesh: &Mesh, faces: &[MeshFace], pick: Pick, tol: f64) -> Result<Found, Unfound> {
    let p = prepare(mesh, weld_tolerance(mesh));
    let mut face_of_given = vec![0u32; mesh.tris.len()];
    for f in faces {
        for &t in &f.triangles {
            if let Some(slot) = face_of_given.get_mut(t as usize) {
                *slot = f.id;
            }
        }
    }
    let face_of: Vec<u32> = p.origin.iter().map(|&o| face_of_given.get(o as usize).copied().unwrap_or(0)).collect();
    let at = Vector3::from(pick.at);
    let seed = (0..p.mesh.tris.len()).filter(|&t| face_of[t] == pick.face).min_by(|&a, &b| (centroid(&p, a) - at).norm().total_cmp(&(centroid(&p, b) - at).norm())).ok_or(Unfound::NoSurface)?;
    let mut t = Tolerance::for_mesh(&p);
    t.distance *= if tol > 0.0 { tol } else { 1.0 };
    t.sharp_deg = SHARP_DEG;
    let region = region_at(&p, &t, seed).ok_or(Unfound::NoSurface)?;
    let mut inside: HashSet<u32> = region.tris.iter().copied().collect();
    let mut surface = region.surface;
    // A CYLINDER THROUGH THE CORNERS OF BROAD FLAT FACES IS NONE. Three corners always lie on some circle: a chamfer and
    // the side of the plate it leans on, 3 and 27 mm long, lay on a cylinder of 21 square to the plate, and the chamfer
    // was taken for a rounding. A wall drawn by flat strips turns little across each: `STRIP_DEG` at the most.
    if let Some(Surface::Cylinder { point, axis, .. }) = &surface {
        let axis = Axis { point: Vector3::from(*point), dir: Vector3::from(*axis).normalize() };
        if inside.iter().any(|&t| turned_deg(&p.mesh.tris[t as usize].map(|i| vertex(&p, i)), axis) > STRIP_DEG) {
            inside = (0..p.mesh.tris.len() as u32).filter(|&t| face_of[t as usize] == pick.face).collect();
            surface = faces.iter().find(|f| f.id == pick.face).map(|f| Surface::Plane { point: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal });
        }
    }
    let look = Look { p: &p, face_of: &face_of, faces, inside: &inside, distance: t.distance };
    match surface {
        Some(Surface::Cylinder { point, axis, radius }) => look.round_wall(Axis { point: point.into(), dir: Vector3::from(axis).normalize() }, radius, at),
        Some(Surface::Plane { normal, .. }) => look.slanted(Vector3::from(normal).normalize()),
        _ => Err(Unfound::Other),
    }
}

/// The diameters of holes a size is brought to: the clearance holes for metric screws, fine and medium, and the
/// drills for tapping them. A printed hole comes out a little smaller than drawn, and its mesh smaller still - a circle
/// drawn by chords - so a measured 3.36 is the 3.4 it was drawn as.
const BORES: [f64; 24] = [2.5, 3.0, 3.2, 3.3, 3.4, 4.2, 4.3, 4.5, 5.0, 5.3, 5.5, 6.4, 6.6, 6.8, 8.4, 8.5, 9.0, 10.2, 10.5, 11.0, 13.0, 14.0, 17.0, 18.0];

/// How far a measured diameter may stand from one of `BORES` to be taken for it, in mm.
const BORE_NEAR: f64 = 0.15;

/// A size measured on a mesh brought to a tenth of a millimetre: what a person typed when the part was drawn.
pub fn rounded(raw: f64) -> f64 {
    (raw * 10.0).round() / 10.0
}

/// A hole's diameter brought to the nearest of `BORES` within `BORE_NEAR`, otherwise to a tenth of a millimetre.
pub fn rounded_bore(raw: f64) -> f64 {
    BORES.iter().copied().filter(|b| (b - raw).abs() <= BORE_NEAR).min_by(|a, b| (a - raw).abs().total_cmp(&(b - raw).abs())).unwrap_or_else(|| rounded(raw))
}

/// An axis: a point on it and its unit direction.
#[derive(Clone, Copy)]
struct Axis {
    point: Vector3<f64>,
    dir: Vector3<f64>,
}

/// A side of the region with a triangle of another face across it: its two corners and that face.
struct Border {
    a: Vector3<f64>,
    b: Vector3<f64>,
    face: u32,
}

/// One end of a hole's wall: the faces met there, and the one it opens through.
#[derive(Default)]
struct End {
    faces: HashSet<u32>,
    opening: Option<u32>,
}

/// Where a side face meets a slanted face: a point, the direction along it, and the side face's normal.
struct SideLine {
    point: Vector3<f64>,
    dir: Vector3<f64>,
    normal: Vector3<f64>,
}

/// What the tests below read: the prepared mesh, the face of every triangle, the region grown at the click.
struct Look<'a> {
    p: &'a Prepared,
    face_of: &'a [u32],
    faces: &'a [MeshFace],
    inside: &'a HashSet<u32>,
    distance: f64,
}

impl Look<'_> {
    /// The faces the region takes: each with more than half of its triangles in it. A triangle of the next face may
    /// lie on the surface along the border, within the tolerance, and does not make that face a part of the feature.
    fn faces_taken(&self) -> Vec<u32> {
        let mut all: HashMap<u32, usize> = HashMap::new();
        let mut taken: HashMap<u32, usize> = HashMap::new();
        for (t, &f) in self.face_of.iter().enumerate() {
            *all.entry(f).or_default() += 1;
            if self.inside.contains(&(t as u32)) {
                *taken.entry(f).or_default() += 1;
            }
        }
        let mut out: Vec<u32> = taken.into_iter().filter(|&(f, n)| f != 0 && 2 * n > all[&f]).map(|(f, _)| f).collect();
        out.sort_unstable();
        out
    }

    /// Every side of the region with a triangle of a face not taken across it.
    fn border(&self, taken: &[u32]) -> Vec<Border> {
        let mut out = Vec::new();
        for &t in self.inside {
            let t = t as usize;
            for k in 0..3 {
                let u = self.p.neighbours[t][k];
                if u == NO_NEIGHBOUR || self.inside.contains(&u) {
                    continue;
                }
                let face = self.face_of[u as usize];
                if face == 0 || taken.contains(&face) {
                    continue;
                }
                let tri = self.p.mesh.tris[t];
                out.push(Border { a: vertex(self.p, tri[k]), b: vertex(self.p, tri[(k + 1) % 3]), face });
            }
        }
        out
    }

    /// The corners of every triangle of face `id`.
    fn corners_of_face(&self, id: u32) -> Vec<Vector3<f64>> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for (t, &f) in self.face_of.iter().enumerate() {
            if f != id {
                continue;
            }
            for &i in &self.p.mesh.tris[t] {
                if seen.insert(i) {
                    out.push(vertex(self.p, i));
                }
            }
        }
        out
    }

    /// The line a side face meets the region along: a point on its longest piece of the border, the piece's
    /// direction, and the face's normal.
    fn side_line(&self, border: &[Border], id: u32) -> Option<SideLine> {
        let b = border.iter().filter(|b| b.face == id).max_by(|x, y| (x.b - x.a).norm().total_cmp(&(y.b - y.a).norm()))?;
        Some(SideLine { point: (b.a + b.b) * 0.5, dir: (b.b - b.a).normalize(), normal: self.face_normal(id)? })
    }

    fn face_normal(&self, id: u32) -> Option<Vector3<f64>> {
        self.faces.iter().find(|f| f.id == id).map(|f| Vector3::from(f.normal))
    }

    /// A cylinder: a hole, a rounding, or neither.
    fn round_wall(&self, axis: Axis, radius: f64, at: Vector3<f64>) -> Result<Found, Unfound> {
        let taken = self.faces_taken();
        let corners: Vec<Vector3<f64>> = self.inside.iter().flat_map(|&t| self.p.mesh.tris[t as usize]).collect::<HashSet<u32>>().into_iter().map(|i| vertex(self.p, i)).collect();
        let turn = turned_deg(&corners, axis);
        // A HOLE'S WALL FACES ITS AXIS: the normals of a solid point out of it, and out of a hole is into the bore
        let facing: f64 = self
            .inside
            .iter()
            .map(|&t| {
                let c = centroid(self.p, t as usize);
                let off = c - axis.point;
                let radial = off - axis.dir * off.dot(&axis.dir);
                normal(self.p, t as usize).dot(&radial.normalize()) * area(self.p, t as usize)
            })
            .sum();
        if turn > ROUND_HOLE {
            if facing >= 0.0 {
                return Err(Unfound::NoEntry);
            }
            return self.hole(axis, radius, at, &corners, taken);
        }
        if turn > ROUND_BLEND {
            return Err(Unfound::Other);
        }
        let sides = self.two_sides(&taken, |b| (b.b - b.a).normalize().dot(&axis.dir).abs() > 0.99)?;
        Ok(Found::Round(Round { radius, sides, faces: taken }))
    }

    /// The faces along the length of the region: exactly two, one each side, or none.
    fn two_sides(&self, taken: &[u32], along: impl Fn(&Border) -> bool) -> Result<EdgeFaces, Unfound> {
        let mut length: HashMap<u32, f64> = HashMap::new();
        for b in self.border(taken) {
            if (b.b - b.a).norm() > self.distance && along(&b) {
                *length.entry(b.face).or_default() += (b.b - b.a).norm();
            }
        }
        let mut by_length: Vec<u32> = length.keys().copied().collect();
        by_length.sort_by(|x, y| length[y].total_cmp(&length[x]).then(x.cmp(y)));
        match by_length.as_slice() {
            [one, other] => Ok(EdgeFaces { one: *one, other: *other }),
            _ => Err(Unfound::NotTwoSides),
        }
    }

    /// A wall going all round its axis: the faces at its two ends tell where it opens and where it stops.
    fn hole(&self, axis: Axis, radius: f64, at: Vector3<f64>, corners: &[Vector3<f64>], mut taken: Vec<u32>) -> Result<Found, Unfound> {
        let height = |v: &Vector3<f64>| (v - axis.point).dot(&axis.dir);
        let low = corners.iter().map(height).fold(f64::INFINITY, f64::min);
        let high = corners.iter().map(height).fold(f64::NEG_INFINITY, f64::max);
        let near = (high - low) * 1e-3 + 10.0 * self.distance;
        let mut bottom = End::default();
        let mut top = End::default();
        for b in self.border(&taken) {
            let (ha, hb) = (height(&b.a), height(&b.b));
            if (ha - low).abs() < near && (hb - low).abs() < near {
                bottom.faces.insert(b.face);
            } else if (ha - high).abs() < near && (hb - high).abs() < near {
                top.faces.insert(b.face);
            }
        }
        // AN END OPENS where a face square to the axis faces away from the bore; it is closed by the faces lying within
        // the bore - a flat bottom, the cone a drill leaves - and those go with the hole
        let within = |id: u32| {
            self.corners_of_face(id).iter().all(|v| {
                let off = v - axis.point;
                (off - axis.dir * off.dot(&axis.dir)).norm() <= radius * (1.0 + 1e-3) + 10.0 * self.distance
            })
        };
        let mut settle = |end: &mut End, out: Vector3<f64>| {
            let mut found: Vec<u32> = end.faces.iter().copied().collect();
            found.sort_unstable();
            for id in found {
                if within(id) {
                    taken.push(id);
                } else if end.opening.is_none() && self.face_normal(id).is_some_and(|n| n.dot(&out) > 0.99) {
                    end.opening = Some(id);
                }
            }
        };
        settle(&mut bottom, -axis.dir);
        settle(&mut top, axis.dir);
        taken.sort_unstable();
        taken.dedup();
        let from_high =
            |entry: u32, reach: Reach| Hole { entry, at: (axis.point + axis.dir * high).into(), inward: (-axis.dir).into(), diameter: 2.0 * radius, depth: high - low, reach, faces: taken.clone() };
        let from_low =
            |entry: u32, reach: Reach| Hole { entry, at: (axis.point + axis.dir * low).into(), inward: axis.dir.into(), diameter: 2.0 * radius, depth: high - low, reach, faces: taken.clone() };
        let hole = match (bottom.opening, top.opening) {
            // through: drilled from the end nearer the place clicked
            (Some(l), Some(h)) => {
                if (height(&at) - high).abs() <= (height(&at) - low).abs() {
                    from_high(h, Reach::Through)
                } else {
                    from_low(l, Reach::Through)
                }
            }
            (None, Some(h)) => from_high(h, Reach::Blind),
            (Some(l), None) => from_low(l, Reach::Blind),
            (None, None) => return Err(Unfound::NoEntry),
        };
        Ok(Found::Hole(hole))
    }

    /// A plane: a chamfer of the edge its two neighbours along its length would meet in, or nothing.
    fn slanted(&self, n: Vector3<f64>) -> Result<Found, Unfound> {
        let taken = self.faces_taken();
        let border = self.border(&taken);
        let leaning = |id: u32| self.face_normal(id).is_some_and(|m| (20.0..=70.0).contains(&n.dot(&m).abs().clamp(-1.0, 1.0).acos().to_degrees()));
        let sides = self.two_sides(&taken, |b| leaning(b.face)).map_err(|_| Unfound::Other)?;
        let (Some(one), Some(other)) = (self.side_line(&border, sides.one), self.side_line(&border, sides.other)) else { return Err(Unfound::Other) };
        if one.dir.dot(&other.dir).abs() < 0.99 {
            return Err(Unfound::Other);
        }
        // THE EDGE THE CHAMFER CUT OFF is where the two side faces meet; each leg is how far its side's line stands from
        // it, measured in that face
        let Some(d) = one.normal.cross(&other.normal).try_normalize(1e-9) else { return Err(Unfound::Other) };
        let m = Matrix3::from_rows(&[one.normal.transpose(), other.normal.transpose(), d.transpose()]);
        let Some(c) = m.lu().solve(&Vector3::new(one.normal.dot(&one.point), other.normal.dot(&other.point), d.dot(&one.point))) else { return Err(Unfound::Other) };
        let leg = |q: Vector3<f64>| {
            let off = q - c;
            (off - d * off.dot(&d)).norm()
        };
        let (l1, l2) = (leg(one.point), leg(other.point));
        if (l1 - l2).abs() > 0.02 * (l1 + l2) * 0.5 + 10.0 * self.distance {
            return Err(Unfound::Uneven);
        }
        Ok(Found::Chamfer(Chamfer { leg: (l1 + l2) * 0.5, sides, faces: taken }))
    }
}

/// How far round `axis` the points go, in degrees: a full turn less the widest gap between them.
fn turned_deg(points: &[Vector3<f64>], axis: Axis) -> f64 {
    let e1 = axis.dir.cross(&if axis.dir.x.abs() < 0.9 { Vector3::x() } else { Vector3::y() }).normalize();
    let e2 = axis.dir.cross(&e1);
    let mut angles: Vec<f64> = points
        .iter()
        .filter_map(|v| {
            let off = v - axis.point;
            let r = off - axis.dir * off.dot(&axis.dir);
            (r.norm() > 1e-12).then(|| r.dot(&e2).atan2(r.dot(&e1)))
        })
        .collect();
    if angles.len() < 2 {
        return 0.0;
    }
    angles.sort_by(f64::total_cmp);
    let wrap = angles[0] + std::f64::consts::TAU - angles[angles.len() - 1];
    let gap = angles.windows(2).map(|w| w[1] - w[0]).fold(wrap, f64::max);
    (std::f64::consts::TAU - gap).to_degrees()
}

fn vertex(p: &Prepared, i: u32) -> Vector3<f64> {
    let v = p.mesh.verts[i as usize];
    Vector3::new(v.x, v.y, v.z)
}

fn centroid(p: &Prepared, t: usize) -> Vector3<f64> {
    let [a, b, c] = p.mesh.tris[t];
    (vertex(p, a) + vertex(p, b) + vertex(p, c)) / 3.0
}

fn normal(p: &Prepared, t: usize) -> Vector3<f64> {
    Vector3::from(p.normals[t])
}

fn area(p: &Prepared, t: usize) -> f64 {
    let [a, b, c] = p.mesh.tris[t];
    (vertex(p, b) - vertex(p, a)).cross(&(vertex(p, c) - vertex(p, a))).norm() * 0.5
}
