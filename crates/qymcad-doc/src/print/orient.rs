//! A BODY LAID ON A FACE: the move that turns the face looking `down` in the world to look straight down, about the
//! middle of the body, then sets the body's lowest point on z = 0 - where a slicer's bed is. The move is in the body's
//! own frame, as every move of a body is: a part placed turned is turned back through its placing, so the face asked
//! in the world is the one that lands.

use qymcad_core::feature::{apply12, apply12_dir, mat_inv12, mat_mul12, rot12_axis};
use qymcad_core::model::{Id, Project};

/// Two directions within this of each other are one: 1e-6 of a turn.
const SAME: f64 = 1e-12;

/// The move that lays `body` on its face looking `down`, a direction in the world; `None` for a body the document does
/// not hold or a direction of no length.
pub fn lay_on(project: &Project, body: Id, down: [f64; 3]) -> Option<[f64; 12]> {
    let len = (down[0] * down[0] + down[1] * down[1] + down[2] * down[2]).sqrt();
    if !(len > SAME && len.is_finite()) {
        return None;
    }
    let mesh = &project.bodies.get(project.mesh_index(body)?)?.mesh;
    let b = mesh.bounds()?;
    let world = project.body_world_transform(body);
    let back = mat_inv12(&world);
    // the face's look and the world's down, both in the body's own frame
    let from = unit(apply12_dir(&back, [down[0] / len, down[1] / len, down[2] / len]));
    let to = unit(apply12_dir(&back, [0.0, 0.0, -1.0]));
    let middle = [(b.min.x + b.max.x) / 2.0, (b.min.y + b.max.y) / 2.0, (b.min.z + b.max.z) / 2.0];
    let cos = (from[0] * to[0] + from[1] * to[1] + from[2] * to[2]).clamp(-1.0, 1.0);
    let axis = cross([from, to]);
    let turn = if cos > 1.0 - SAME {
        qymcad_core::feature::PLACE_IDENTITY
    } else if dot([axis, axis]) < SAME {
        // looking straight up: half a turn about any line across it
        let across = if from[0].abs() < 0.9 { cross([from, [1.0, 0.0, 0.0]]) } else { cross([from, [0.0, 1.0, 0.0]]) };
        rot12_axis(middle, across, 180.0)
    } else {
        rot12_axis(middle, axis, cos.acos().to_degrees())
    };
    // the lowest point of the body so turned, in the world, set on z = 0 by a shift down the world's upright
    let placed = mat_mul12(&world, &turn);
    let lowest = mesh.verts.iter().map(|v| apply12(&placed, [v.x, v.y, v.z])[2]).fold(f64::INFINITY, f64::min);
    let shift = apply12_dir(&back, [0.0, 0.0, -lowest]);
    let mut m = turn;
    m[3] += shift[0];
    m[7] += shift[1];
    m[11] += shift[2];
    Some(m)
}

/// The dot product of a pair of directions.
fn dot([a, b]: [[f64; 3]; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The cross product of a pair of directions, the first times the second.
fn cross([a, b]: [[f64; 3]; 2]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = dot([a, a]).sqrt().max(1e-300);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// A way to lay a body, rated as it would then print.
#[derive(Clone, Debug, PartialEq)]
pub struct Way {
    /// The way the face laid on looks now, in the world: `lay_on` takes it.
    pub down: [f64; 3],
    /// The key of the face, where the way is a face's of the body's own; none for a way along an axis.
    pub face: Option<u32>,
    /// What would hang over the air, mm^2.
    pub overhang: f64,
    /// What would stand on the bed, mm^2.
    pub contact: f64,
    /// How tall it would stand, mm.
    pub height: f64,
}

/// A way tried: the face it is of (0 for an axis) and the way that face looks, in the world.
struct Tried {
    face: u32,
    down: [f64; 3],
}

/// The faces whose ways are tried at most, the broadest first: past six a body's flat faces are small ones, the
/// sides of its steps and its slots.
const FACES: usize = 6;

/// The ways told at most.
const WAYS: usize = 3;

/// Two ways within this of each other are one: 1 deg.
const ONE_WAY: f64 = 0.999_847_7;

/// THE BEST WAYS TO LAY `body`, the best first: each of its broadest flat faces looking down, and each way along the
/// world's axes, rated as it would print - the least hanging over the air past `angle_deg` from the upright first, of
/// those the most standing on the bed, of those the lowest. A way that stands on no area at all - an edge, a point - is
/// told last. Nothing is changed.
pub fn suggest(project: &Project, body: Id, angle_deg: f64) -> Vec<Way> {
    let Some(mi) = project.mesh_index(body) else { return Vec::new() };
    let world = project.body_world_transform(body);
    let faces = project.regen_faces.get(&body).map(Vec::as_slice).unwrap_or(project.bodies[mi].faces.as_slice());
    let mut broad: Vec<&qymcad_core::geom::MeshFace> = faces.iter().collect();
    broad.sort_by(|a, b| b.area.total_cmp(&a.area));
    let mut tried: Vec<Tried> = Vec::new();
    let seen = |d: [f64; 3], tried: &[Tried]| tried.iter().any(|t| dot([t.down, d]) > ONE_WAY);
    for f in broad {
        if tried.len() >= FACES {
            break;
        }
        let down = unit(apply12_dir(&world, f.normal));
        if !seen(down, &tried) {
            tried.push(Tried { face: f.id, down });
        }
    }
    for down in [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]] {
        if !seen(down, &tried) {
            tried.push(Tried { face: 0, down });
        }
    }
    let mesh = &project.bodies[mi].mesh;
    let mut ways: Vec<Way> = tried
        .into_iter()
        .filter_map(|Tried { face, down }| {
            let m = lay_on(project, body, down)?;
            let placed = mat_mul12(&world, &m);
            let mut laid = mesh.clone();
            for v in &mut laid.verts {
                let w = apply12(&placed, [v.x, v.y, v.z]);
                *v = qymcad_core::geom::Point3::new(w[0], w[1], w[2]);
            }
            let b = laid.bounds()?;
            Some(Way {
                down,
                face: (face != 0).then_some(face),
                overhang: crate::print::checks::overhang(&laid, angle_deg).area,
                contact: crate::print::checks::contact(&laid).area,
                height: b.max.z - b.min.z,
            })
        })
        .collect();
    // to a hundredth of a mm^2 and a micron: a mesh's rounding does not put one way before another
    let key = |w: &Way| ((w.contact <= 1e-6) as u8, (w.overhang * 100.0).round() as i64, -(w.contact * 100.0).round() as i64, (w.height * 1000.0).round() as i64);
    ways.sort_by_key(key);
    ways.truncate(WAYS);
    ways
}
