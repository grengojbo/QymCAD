//! WHAT A FACE OR AN EDGE IS, FOR A MEASUREMENT: a plane or a cylinder, a line or a circle, placed by the transform it
//! is given. The window measures what is clicked and the protocol server what is asked for; both turn a face or an
//! edge of a live body into the same `MeasureItem`, and the numbers come from `qymcad_core::measure`.

use qymcad_core::feature::{apply12, apply12_dir, FaceKey};
use qymcad_core::measure::MeasureItem;
use qymcad_core::model::{Id, Project};
use qymcad_kernel::Shape;

/// A FACE: a cylinder when the kernel reads it as one - the wall of a hole is measured for its diameter and for the gap
/// to that wall, not to a plane it does not have - and otherwise a plane through its centre along its normal. With no
/// live shape to ask, a plane.
pub fn face_item(project: &Project, shape: Option<&Shape>, body: Id, face: u32, place: &[f64; 12]) -> MeasureItem {
    if let Some((o, axis, r)) = shape.and_then(|s| s.face_cylinder(face)) {
        return MeasureItem::Cylinder { origin: apply12(place, o), axis: apply12_dir(place, axis), r };
    }
    let key = FaceKey { index: 0, centroid: [0.0; 3], normal: [0.0, 0.0, 1.0], id: face };
    let (c, n) = project.resolve_face(body, &key);
    MeasureItem::Plane { origin: apply12(place, c), normal: apply12_dir(place, n) }
}

/// AN EDGE: a circle when it is one - a polyline from the tessellation would give the rim of a hole a length instead of
/// a diameter - and otherwise a line from its first point to its last, as long as the polyline runs (an arc has a
/// shorter chord). `None` when the shape has no such edge or the edge has no points.
pub fn edge_item(shape: &Shape, edge: u32, place: &[f64; 12]) -> Option<MeasureItem> {
    let (polys, ids, circles) = shape.edges_full();
    let k = ids.iter().position(|x| *x == edge)?;
    if let Some((c, axis, r)) = circles[k] {
        return Some(MeasureItem::Circle { center: apply12(place, c), axis: apply12_dir(place, axis), r });
    }
    let poly: Vec<[f64; 3]> = polys[k].iter().map(|p| apply12(place, [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])])).collect();
    let (a, b) = (*poly.first()?, *poly.last()?);
    if poly.len() < 2 {
        return None;
    }
    let len: f64 = poly.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2) + (w[1][2] - w[0][2]).powi(2)).sqrt()).sum();
    Some(MeasureItem::Line { origin: a, dir: [b[0] - a[0], b[1] - a[1], b[2] - a[2]], len })
}

/// The point an item is shown at: the centre of a circle, the middle of a line, the origin of a face.
pub fn shown_at(item: &MeasureItem) -> [f64; 3] {
    match *item {
        MeasureItem::Point(p) => p,
        MeasureItem::Line { origin, dir, .. } => [origin[0] + dir[0] * 0.5, origin[1] + dir[1] * 0.5, origin[2] + dir[2] * 0.5],
        MeasureItem::Circle { center, .. } => center,
        MeasureItem::Plane { origin, .. } | MeasureItem::Cylinder { origin, .. } => origin,
    }
}
