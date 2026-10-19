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

use crate::{cylinder_of, prepare, region_at, weld_tolerance, Prepared, Region, Surface, Tolerance, NO_NEIGHBOUR};
use nalgebra::{Matrix3, Vector3};
use qymcad_core::feature::FaceKey;
use qymcad_core::geom::{EdgeFaces, Mesh, MeshFace};
pub use qymcad_core::geom::{rounded, rounded_bore};
pub use qymcad_core::model::Reach;
use qymcad_core::model::{Exact, Remake, RemakeGroup};
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

/// A FACE WHOSE TRIANGLES TURN MORE THAN THIS, in degrees, is curved: a face of an exact surface, or a smooth wall a
/// recognition fitted. A flat face of a polyhedron turns none.
const CURVED_DEG: f64 = 2.0;

/// HOW FAR A CURVED FACE MAY STAND OFF ITS CYLINDER, as a share of the radius. A recognition that took a hole drawn by
/// chords for a smooth wall left it between the chords and the circle: measured on a hole of 10 meshed at 0.1, its
/// corners stood 4.986 to 5.000 from the axis - 0.3% - where the distance tolerance of the mesh is 0.0005 mm, and the
/// wall grew no further than 10 of its 310 triangles. A chord of a circle drawn with 16 sides sags 1.9% of it.
const ROUND_OFF: f64 = 0.03;

/// A FLAT FACE WIDER THAN THIS MANY OF THE STRIP CLICKED is never a strip of the same wall. The strips of one wall are
/// cut alike, and a side of the part beside a rounding drawn with 16 strips, turned 2.8 deg from the first of them, lay
/// with it on a cylinder of 370 and took the rounding: 360 mm^2 against 3.9.
const STRIP_AREA: f64 = 4.0;

/// The place a person clicked: the face under the pointer, by its persistent id, and the point on it.
#[derive(Clone, Copy, Debug)]
pub struct Pick {
    pub face: u32,
    pub at: [f64; 3],
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
    Body::new(mesh, faces, tol).at(pick)
}

/// EVERYTHING FOUND ON A BODY: its features, no face in two of them, and the curved faces none of them took - what
/// was left as it is.
#[derive(Clone, Debug, Default)]
pub struct Everything {
    pub found: Vec<Found>,
    pub unfound: Vec<u32>,
}

/// EVERY HOLE, ROUNDING AND CHAMFER OF THE BODY of `mesh`, each looked for from a face no feature has taken yet, as a
/// click at its centre would, the body read once for all of them. A rounding of a chain leaves the corners where it
/// runs round another rounding to no feature: such a corner goes with the rounding of the smallest radius beside it,
/// the one laid last, which makes it again.
pub fn features_of(mesh: &Mesh, faces: &[MeshFace], tol: f64) -> Everything {
    let body = Body::new(mesh, faces, tol);
    let mut taken: HashSet<u32> = HashSet::new();
    let mut found: Vec<Found> = Vec::new();
    for f in faces {
        if taken.contains(&f.id) {
            continue;
        }
        let Ok(x) = body.at(Pick { face: f.id, at: [f.centroid.x, f.centroid.y, f.centroid.z] }) else { continue };
        if x.faces().iter().any(|id| taken.contains(id)) {
            continue;
        }
        taken.extend(x.faces().iter().copied());
        found.push(x);
    }
    for id in body.pieces_of_corners(&mut found) {
        taken.remove(&id);
    }
    let mut found = one_a_edge(found);
    let unfound = body.corners_of_chains(&mut found, &taken);
    Everything { found, unfound }
}

/// THE FEATURES FOUND, MADE AGAIN EXACT, IN GROUPS OF ONE SIZE: a group holds the features of one kind whose sizes,
/// brought to what was drawn, are the same - a blind hole its depth as well - so one field changes them all. The holes
/// come first, then the chamfers, then the roundings, each kind from its smaller size up. A feature naming a face the
/// body's list does not hold is left out.
pub fn grouped(found: &[Found], faces: &[MeshFace]) -> Vec<RemakeGroup> {
    let mut groups: Vec<RemakeGroup> = Vec::new();
    let mut made: Vec<Remake> = found.iter().filter_map(|f| f.remake(faces)).collect();
    made.sort_by(|a, b| {
        let (ka, kb) = (kin(&a.exact), kin(&b.exact));
        ka.order.cmp(&kb.order).then(ka.size.total_cmp(&kb.size))
    });
    for r in made {
        match groups.iter_mut().find(|g| g.items.first().is_some_and(|first| kin(&first.exact) == kin(&r.exact))) {
            Some(g) => g.items.push(r),
            None => groups.push(RemakeGroup { items: vec![r] }),
        }
    }
    groups
}

/// WHAT A GROUP IS TOLD BY: the place of its kind among the groups - holes, chamfers, roundings - its size, brought to
/// what was drawn, and the depth of a blind hole; a hole through is drilled past the body whatever its depth.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Kin {
    order: u8,
    size: f64,
    depth: Option<f64>,
}

fn kin(e: &Exact) -> Kin {
    match *e {
        Exact::Hole { diameter, depth, reach, .. } => Kin { order: 0, size: diameter, depth: (reach == Reach::Blind).then_some(depth) },
        Exact::Chamfer { leg, .. } => Kin { order: 1, size: leg, depth: None },
        Exact::Round { radius, .. } => Kin { order: 2, size: radius, depth: None },
    }
}

/// HOW FAR TWO PIECES OF ONE BLEND MAY DIFFER IN SIZE, as a share of it. A rounding drawn with strips is found from
/// each run of them on its own circle: measured on a rounding of 2, from 1.99 to 2.01.
const PIECE_SIZE: f64 = 0.05;

/// ONE BLEND AN EDGE: the pieces of one rounding or chamfer - lying between the same two faces, their sizes within
/// `PIECE_SIZE` - are one, its faces all theirs, its size the first piece's. A rounding running round a corner of a
/// mesh drawn with strips is found piece by piece, and laid again an edge goes in once.
fn one_a_edge(found: Vec<Found>) -> Vec<Found> {
    let same_edge = |a: &EdgeFaces, b: &EdgeFaces| (a.one == b.one && a.other == b.other) || (a.one == b.other && a.other == b.one);
    let close = |x: f64, y: f64| (x - y).abs() <= PIECE_SIZE * x.max(y);
    let mut out: Vec<Found> = Vec::new();
    for f in found {
        let home = out.iter_mut().find(|g| match (&**g, &f) {
            (Found::Round(a), Found::Round(b)) => same_edge(&a.sides, &b.sides) && close(a.radius, b.radius),
            (Found::Chamfer(a), Found::Chamfer(b)) => same_edge(&a.sides, &b.sides) && close(a.leg, b.leg),
            _ => false,
        });
        match (home, f) {
            (Some(Found::Round(a)), Found::Round(b)) => {
                a.faces.extend(b.faces);
                a.faces.sort_unstable();
            }
            (Some(Found::Chamfer(a)), Found::Chamfer(b)) => {
                a.faces.extend(b.faces);
                a.faces.sort_unstable();
            }
            (_, f) => out.push(f),
        }
    }
    out
}

/// A BODY READ FOR A LOOK AT IT: its mesh prepared, the face of every triangle, the triangles of every face, the
/// tolerance of the look.
struct Body<'a> {
    p: Prepared,
    face_of: Vec<u32>,
    by_face: HashMap<u32, Vec<u32>>,
    faces: &'a [MeshFace],
    t: Tolerance,
}

impl<'a> Body<'a> {
    fn new(mesh: &Mesh, faces: &'a [MeshFace], tol: f64) -> Body<'a> {
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
        let mut t = Tolerance::for_mesh(&p);
        t.distance *= if tol > 0.0 { tol } else { 1.0 };
        t.sharp_deg = SHARP_DEG;
        let mut by_face: HashMap<u32, Vec<u32>> = HashMap::new();
        for (tri, &f) in face_of.iter().enumerate() {
            by_face.entry(f).or_default().push(tri as u32);
        }
        Body { p, face_of, by_face, faces, t }
    }

    /// The feature at `pick`.
    fn at(&self, pick: Pick) -> Result<Found, Unfound> {
        let p = &self.p;
        let at = Vector3::from(pick.at);
        let mine = self.by_face.get(&pick.face).cloned().unwrap_or_default();
        let seed = mine.iter().map(|&t| t as usize).min_by(|&a, &b| (centroid(p, a) - at).norm().total_cmp(&(centroid(p, b) - at).norm())).ok_or(Unfound::NoSurface)?;
        let wall = if curved(p, &mine) {
            round_faces(p, &self.by_face, &self.face_of, pick.face, &self.t)?
        } else {
            strips(p, &self.by_face, &self.face_of, seed, pick.face, &self.t)?
        };
        let inside: HashSet<u32> = wall.tris.iter().copied().collect();
        let look = Look { p, face_of: &self.face_of, faces: self.faces, inside: &inside, distance: self.t.distance };
        match wall.surface {
            Some(Surface::Cylinder { point, axis, radius }) => look.round_wall(Axis { point: point.into(), dir: Vector3::from(axis).normalize() }, radius, at),
            Some(Surface::Plane { normal, .. }) => look.slanted(Vector3::from(normal).normalize()),
            _ => Err(Unfound::Other),
        }
    }

    /// The faces across the edges of the triangles of `group`, not in it.
    fn beside(&self, group: &HashSet<u32>) -> HashSet<u32> {
        let mut out = HashSet::new();
        for &f in group {
            for &tri in self.by_face.get(&f).map(Vec::as_slice).unwrap_or_default() {
                for &u in &self.p.neighbours[tri as usize] {
                    let g = if u == NO_NEIGHBOUR { 0 } else { self.face_of[u as usize] };
                    if g != 0 && !group.contains(&g) {
                        out.insert(g);
                    }
                }
            }
        }
        out
    }

    /// The faces across the smooth edges of face `f` - its triangle and theirs turned less than the sharp angle - as a
    /// curved wall goes on across them and a corner of a body does not.
    fn smoothly_beside(&self, f: u32) -> HashSet<u32> {
        let cos_sharp = self.t.sharp_deg.to_radians().cos();
        let mut out = HashSet::new();
        for &tri in self.by_face.get(&f).map(Vec::as_slice).unwrap_or_default() {
            for &u in &self.p.neighbours[tri as usize] {
                if u == NO_NEIGHBOUR || normal(&self.p, tri as usize).dot(&normal(&self.p, u as usize)) < cos_sharp {
                    continue;
                }
                let g = self.face_of[u as usize];
                if g != 0 && g != f {
                    out.insert(g);
                }
            }
        }
        out
    }

    /// THE ROUNDINGS THAT ARE PIECES OF A CORNER, taken out of `found` and their faces returned. Where a rounding of a
    /// chain runs round another, its surface turns about two axes, and a mesh drawn with flat strips - or recognised
    /// into smaller walls - gives pieces of it that each lie on some cylinder of the radius: measured on a block whose
    /// upright edges are rounded 4 and whose top edge is rounded 2 all round, 30 such pieces besides the 4 straight
    /// roundings of 2. A piece has for a side a rounding, another piece, or a facet of the corner smaller across than
    /// the radius - a rounding of an edge eats at least its radius into each of its two faces. The pieces go back to
    /// the corner they are of (`corners_of_chains`).
    fn pieces_of_corners(&self, found: &mut Vec<Found>) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        loop {
            let curved: HashSet<u32> = found.iter().filter(|f| matches!(f, Found::Round(_))).flat_map(|f| f.faces().iter().copied()).chain(out.iter().copied()).collect();
            let piece = found.iter().position(|f| match f {
                Found::Round(r) => [r.sides.one, r.sides.other].iter().any(|s| (curved.contains(s) && !r.faces.contains(s)) || self.across(*s) < r.radius),
                _ => false,
            });
            let Some(i) = piece else { return out };
            out.extend(found.remove(i).faces().iter().copied());
        }
    }

    /// How far apart the two corners of face `f` farthest from each other lie.
    fn across(&self, f: u32) -> f64 {
        let corners: Vec<Vector3<f64>> = self.by_face.get(&f).map(Vec::as_slice).unwrap_or_default().iter().flat_map(|&t| self.p.mesh.tris[t as usize]).map(|i| vertex(&self.p, i)).collect();
        corners.iter().flat_map(|a| corners.iter().map(move |b| (a - b).norm())).fold(0.0, f64::max)
    }

    /// THE CORNERS OF THE CHAINS OF ROUNDINGS go with the rounding of the smallest radius they touch; the curved faces
    /// left after that are returned. A corner is a group of faces meeting each other across smooth edges, taken by no feature and none of
    /// them a side of a rounding, curved as a whole - a flat face of the body touching only roundings and their sides
    /// is no corner - and touching nothing but roundings and their sides, a rounding at least. A corner of a polyhedron
    /// is a group of flat facets, curved as a whole as an exact corner is.
    fn corners_of_chains(&self, found: &mut [Found], taken: &HashSet<u32>) -> Vec<u32> {
        let mut rounds: HashMap<u32, usize> = HashMap::new();
        let mut sides: HashSet<u32> = HashSet::new();
        for (i, f) in found.iter().enumerate() {
            if let Found::Round(r) = f {
                rounds.extend(r.faces.iter().map(|&id| (id, i)));
                sides.extend([r.sides.one, r.sides.other]);
            }
        }
        let free: Vec<u32> = self.faces.iter().map(|f| f.id).filter(|id| *id != 0 && !taken.contains(id) && !sides.contains(id)).collect();
        let mut seen: HashSet<u32> = HashSet::new();
        let mut unfound = Vec::new();
        for &start in &free {
            if !seen.insert(start) {
                continue;
            }
            let mut group = HashSet::from([start]);
            let mut queue = vec![start];
            while let Some(f) = queue.pop() {
                for g in self.smoothly_beside(f) {
                    if free.contains(&g) && seen.insert(g) {
                        group.insert(g);
                        queue.push(g);
                    }
                }
            }
            let tris: Vec<u32> = group.iter().flat_map(|f| self.by_face.get(f).cloned().unwrap_or_default()).collect();
            if !curved(&self.p, &tris) {
                continue;
            }
            let round = self.beside(&group);
            let laid_last = round.iter().filter_map(|g| rounds.get(g).copied()).min_by(|&a, &b| radius_of(&found[a]).total_cmp(&radius_of(&found[b])).then(a.cmp(&b)));
            match laid_last {
                Some(i) if round.iter().all(|g| rounds.contains_key(g) || sides.contains(g)) => {
                    if let Found::Round(r) = &mut found[i] {
                        r.faces.extend(group.iter().copied());
                        r.faces.sort_unstable();
                    }
                }
                _ => unfound.extend(group.iter().copied()),
            }
        }
        unfound.sort_unstable();
        unfound
    }
}

/// The radius of a rounding; nothing else is one.
fn radius_of(f: &Found) -> f64 {
    match f {
        Found::Round(r) => r.radius,
        _ => f64::INFINITY,
    }
}

impl Found {
    /// The faces of the body it is made of.
    pub fn faces(&self) -> &[u32] {
        match self {
            Found::Hole(h) => &h.faces,
            Found::Round(r) => &r.faces,
            Found::Chamfer(c) => &c.faces,
        }
    }

    /// The feature made again exact, as the core lays it down (`Project::add_remade_feature`): its faces as keys of
    /// `faces` - the body's own list - and its sizes brought to what was drawn. A hole through is drilled twice its
    /// depth and a millimetre more, so that it still goes through where the body grows thicker above it.
    pub fn remake(&self, faces: &[MeshFace]) -> Option<Remake> {
        let key = |id: u32| faces.iter().position(|f| f.id == id).map(|i| face_key(i, &faces[i]));
        let keys = |ids: &[u32]| ids.iter().map(|&id| key(id)).collect::<Option<Vec<FaceKey>>>();
        Some(match self {
            Found::Hole(h) => {
                let depth = match h.reach {
                    Reach::Through => 2.0 * h.depth + 1.0,
                    Reach::Blind => rounded(h.depth),
                };
                Remake { faces: keys(&h.faces)?, exact: Exact::Hole { entry: key(h.entry)?, at: h.at, diameter: rounded_bore(h.diameter), depth, reach: h.reach } }
            }
            Found::Round(r) => Remake { faces: keys(&r.faces)?, exact: Exact::Round { sides: r.sides, radius: rounded(r.radius) } },
            Found::Chamfer(c) => Remake { faces: keys(&c.faces)?, exact: Exact::Chamfer { sides: c.sides, leg: rounded(c.leg) } },
        })
    }
}

/// The key of face `i` of a body's list, as a pick gives it.
fn face_key(i: usize, f: &MeshFace) -> FaceKey {
    FaceKey { index: i as u32, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id }
}

/// Whether the triangles of a face turn more than `CURVED_DEG` from each other.
fn curved(p: &Prepared, tris: &[u32]) -> bool {
    let cos = CURVED_DEG.to_radians().cos();
    tris.first().is_some_and(|&a| tris.iter().any(|&b| normal(p, a as usize).dot(&normal(p, b as usize)) < cos))
}

/// A CURVED FACE CLICKED IS TAKEN WHOLE: the cylinder through all its corners, and the curved faces beside it across a
/// smooth edge lying on that cylinder too - a wall a kernel parted at its seam. A face standing off its cylinder by more
/// than `ROUND_OFF` of the radius is no round wall.
fn round_faces(p: &Prepared, by_face: &HashMap<u32, Vec<u32>>, face_of: &[u32], clicked: u32, t: &Tolerance) -> Result<Region, Unfound> {
    let mut tris = by_face.get(&clicked).cloned().unwrap_or_default();
    let surface = cylinder_of(p, &tris, t).ok_or(Unfound::Other)?;
    let Surface::Cylinder { radius, .. } = surface else { return Err(Unfound::Other) };
    let off = |tris: &[u32]| tris.iter().flat_map(|&tri| p.mesh.tris[tri as usize]).map(|i| surface.distance(vertex(p, i).into())).fold(0.0, f64::max);
    let room = (ROUND_OFF * radius).max(t.distance);
    if off(&tris) > room {
        return Err(Unfound::Other);
    }
    let cos_sharp = t.sharp_deg.to_radians().cos();
    let mut taken = HashSet::from([clicked]);
    let mut queue = vec![clicked];
    while let Some(f) = queue.pop() {
        for &tri in by_face.get(&f).map(Vec::as_slice).unwrap_or_default() {
            for &u in &p.neighbours[tri as usize] {
                if u == NO_NEIGHBOUR || normal(p, tri as usize).dot(&normal(p, u as usize)) < cos_sharp {
                    continue;
                }
                let g = face_of[u as usize];
                let next = by_face.get(&g).map(Vec::as_slice).unwrap_or_default();
                if g == 0 || taken.contains(&g) || !curved(p, next) || off(next) > room {
                    continue;
                }
                taken.insert(g);
                queue.push(g);
                tris.extend_from_slice(next);
            }
        }
    }
    Ok(Region { tris, surface: Some(surface) })
}

/// A FLAT FACE CLICKED IS A STRIP OF A WALL DRAWN FLAT, OR A FLAT FACE: the surface is grown from the triangle clicked
/// over the strips beside it (`region_at`), the faces wider than `STRIP_AREA` strips barred. A cylinder in which one
/// triangle turns more than `STRIP_DEG` about its axis is none - three corners of two broad faces always lie on some
/// circle: a chamfer and the side of the plate it leans on, 3 and 27 mm long, lay on a cylinder of 21 - and the face
/// clicked is then taken as the plane it is.
fn strips(p: &Prepared, by_face: &HashMap<u32, Vec<u32>>, face_of: &[u32], seed: usize, clicked: u32, t: &Tolerance) -> Result<Region, Unfound> {
    let face_area = |f: u32| by_face.get(&f).map(Vec::as_slice).unwrap_or_default().iter().map(|&tri| area(p, tri as usize)).sum::<f64>();
    let mine = face_area(clicked);
    let wide: HashSet<u32> = by_face.keys().copied().filter(|&f| f != clicked && face_area(f) > STRIP_AREA * mine).collect();
    let barred: Vec<bool> = face_of.iter().map(|f| wide.contains(f)).collect();
    let region = region_at(p, t, seed, &barred).ok_or(Unfound::NoSurface)?;
    if let Some(Surface::Cylinder { point, axis, .. }) = &region.surface {
        let axis = Axis { point: Vector3::from(*point), dir: Vector3::from(*axis).normalize() };
        if region.tris.iter().any(|&tri| turned_deg(&p.mesh.tris[tri as usize].map(|i| vertex(p, i)), axis) > STRIP_DEG) {
            let face = by_face.get(&clicked).cloned().unwrap_or_default();
            let n = face.first().map(|&tri| normal(p, tri as usize)).ok_or(Unfound::NoSurface)?;
            let c = face.first().map(|&tri| centroid(p, tri as usize)).ok_or(Unfound::NoSurface)?;
            return Ok(Region { tris: face, surface: Some(Surface::Plane { point: c.into(), normal: n.into() }) });
        }
    }
    Ok(region)
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
