//! WHETHER A BODY PRINTS AS IT LIES: whether it fits the bed, how much of it stands on the bed, and where it hangs over
//! the air. Read on the body's triangles placed in the world, the bed being the plane through the body's lowest point:
//! a slicer drops a part onto the bed, it does not print it where it floats.

use qymcad_core::geom::Mesh;

/// What a printer prints in: the sides of the room it builds, mm - not of its plate, which can be wider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bed {
    pub size: [f64; 3],
}

/// How a check came out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    Pass,
    /// It prints, with something to mind: turned to fit, held by supports.
    Warn,
    /// It does not print as it lies.
    Fail,
}

/// How a body fits the bed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    AsItLies,
    /// Only turned a quarter about the upright: its two sides across swapped.
    Turned,
    Not,
}

/// The body against the bed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fits {
    pub verdict: Verdict,
    pub fit: Fit,
    /// The sides of the body's box, mm.
    pub size: [f64; 3],
}

/// Where a body stands on the bed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub verdict: Verdict,
    /// The area of its faces lying flat on the bed, mm^2: none is a body on a point or an edge, which the first layer
    /// cannot hold.
    pub area: f64,
}

/// A place of a body hanging over the air.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// The middle of the place, in the world.
    pub at: [f64; 3],
    pub area: f64,
}

/// Where a body hangs over the air past what a printer bridges.
#[derive(Clone, Debug, PartialEq)]
pub struct Overhang {
    pub verdict: Verdict,
    /// The area facing down past the angle, mm^2, the bottom on the bed left out.
    pub area: f64,
    /// The places it hangs at, joined across the sides they share, the widest first; at most `PLACES`.
    pub places: Vec<Place>,
}

/// The places an answer names at most: past ten a reader does not look at them one by one.
pub const PLACES: usize = 10;

/// How far above the lowest point a corner still lies on the bed, mm: a mesh written in single precision places a
/// corner of a flat bottom to a few tenths of a micron.
const ON_BED: f64 = 1e-3;

/// How far a face may turn from facing straight down and still lie flat on the bed: 1 deg.
const FLAT_DOWN: f64 = 0.999_847_7;

/// An area under this, mm^2, is none: the rounding of a mesh, not a place.
const NONE: f64 = 1e-6;

/// The box of the mesh's corners.
fn bounds(m: &Mesh) -> Option<[[f64; 3]; 2]> {
    let b = m.bounds()?;
    Some([[b.min.x, b.min.y, b.min.z], [b.max.x, b.max.y, b.max.z]])
}

/// Whether `mesh` fits `bed`, as it lies or turned a quarter about the upright.
pub fn fits(mesh: &Mesh, bed: &Bed) -> Fits {
    let Some([lo, hi]) = bounds(mesh) else { return Fits { verdict: Verdict::Fail, fit: Fit::Not, size: [0.0; 3] } };
    let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    let within = |a: f64, b: f64| a <= b + ON_BED;
    let tall = within(size[2], bed.size[2]);
    let fit = if tall && within(size[0], bed.size[0]) && within(size[1], bed.size[1]) {
        Fit::AsItLies
    } else if tall && within(size[0], bed.size[1]) && within(size[1], bed.size[0]) {
        Fit::Turned
    } else {
        Fit::Not
    };
    let verdict = match fit {
        Fit::AsItLies => Verdict::Pass,
        Fit::Turned => Verdict::Warn,
        Fit::Not => Verdict::Fail,
    };
    Fits { verdict, fit, size }
}

/// The lowest height of the mesh: where the bed stands.
fn floor(m: &Mesh) -> f64 {
    m.verts.iter().map(|v| v.z).fold(f64::INFINITY, f64::min)
}

/// Whether triangle `t` lies on the bed at `z0`: all its corners there.
fn on_bed(m: &Mesh, t: usize, z0: f64) -> bool {
    m.triangle(t).iter().all(|p| p.z <= z0 + ON_BED)
}

/// How much of `mesh` stands on the bed.
pub fn contact(mesh: &Mesh) -> Contact {
    let z0 = floor(mesh);
    let area: f64 = (0..mesh.tris.len()).filter(|&t| on_bed(mesh, t, z0)).map(|t| mesh.tri_normal_area(t)).filter(|(n, _, _)| n[2] < -FLAT_DOWN).map(|(_, a, _)| a).sum();
    Contact { verdict: if area > NONE { Verdict::Pass } else { Verdict::Fail }, area }
}

/// Where `mesh` hangs over the air: its faces turned down more than `angle_deg` from the upright, the bottom on the bed
/// left out. 45 deg is what a printer bridges without supports; a face turned past it sags.
pub fn overhang(mesh: &Mesh, angle_deg: f64) -> Overhang {
    let z0 = floor(mesh);
    let limit = -angle_deg.to_radians().sin();
    let hanging: Vec<usize> = (0..mesh.tris.len()).filter(|&t| !on_bed(mesh, t, z0) && mesh.tri_normal_area(t).0[2] < limit).collect();
    let area: f64 = hanging.iter().map(|&t| mesh.tri_normal_area(t).1).sum();
    let mut places = places_of(mesh, &hanging);
    places.sort_by(|a, b| b.area.total_cmp(&a.area));
    places.truncate(PLACES);
    Overhang { verdict: if area > NONE { Verdict::Warn } else { Verdict::Pass }, area, places }
}

/// The triangles `tris` joined into places across the sides they share by where those stand, each its area and the
/// middle of it weighted by area.
fn places_of(mesh: &Mesh, tris: &[usize]) -> Vec<Place> {
    let mut root: Vec<usize> = (0..tris.len()).collect();
    fn find(root: &mut [usize], mut i: usize) -> usize {
        while root[i] != i {
            root[i] = root[root[i]];
            i = root[i];
        }
        i
    }
    let place = |v: u32| {
        let p = mesh.verts[v as usize];
        [(p.x / 1e-6).round() as i64, (p.y / 1e-6).round() as i64, (p.z / 1e-6).round() as i64]
    };
    let mut first: std::collections::HashMap<[i64; 6], usize> = std::collections::HashMap::new();
    for (i, &t) in tris.iter().enumerate() {
        let tri = mesh.tris[t];
        for k in 0..3 {
            let (a, b) = (place(tri[k]), place(tri[(k + 1) % 3]));
            let side = if a < b { [a[0], a[1], a[2], b[0], b[1], b[2]] } else { [b[0], b[1], b[2], a[0], a[1], a[2]] };
            match first.get(&side) {
                Some(&j) => {
                    let (x, y) = (find(&mut root, i), find(&mut root, j));
                    if x != y {
                        root[x] = y;
                    }
                }
                None => {
                    first.insert(side, i);
                }
            }
        }
    }
    let mut by_root: std::collections::BTreeMap<usize, Place> = std::collections::BTreeMap::new();
    for (i, &t) in tris.iter().enumerate() {
        let (_, a, c) = mesh.tri_normal_area(t);
        let r = find(&mut root, i);
        let p = by_root.entry(r).or_insert(Place { at: [0.0; 3], area: 0.0 });
        p.at = [p.at[0] + c.x * a, p.at[1] + c.y * a, p.at[2] + c.z * a];
        p.area += a;
    }
    by_root.into_values().map(|p| Place { at: p.at.map(|v| v / p.area.max(1e-300)), area: p.area }).collect()
}
