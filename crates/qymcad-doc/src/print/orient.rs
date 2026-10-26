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
