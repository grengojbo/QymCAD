//! A BODY CUT TO FIT THE BED: planes across each side the room of the printer does not hold, spaced evenly so every
//! piece is the room less a margin at most, each plane cutting every piece it runs through. The pieces stay bodies of
//! the one part, as every cut leaves them, and the cuts are one step.

use crate::print::checks::Bed;
use crate::{DocEngine, DocError};
use qymcad_core::model::Id;

/// A cut: the axis it runs across (0 for X, 1 for Y, 2 for Z) and where along it, in the body's own frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    pub axis: usize,
    pub at: f64,
}

/// The cuts that bring a body of the box `bounds` - its lowest and highest corner - within `bed` less `margin` on each
/// side: as few pieces along each side as fit, cut evenly.
pub fn cuts_to_fit(bounds: [[f64; 3]; 2], bed: &Bed, margin: f64) -> Vec<Cut> {
    let [lo, hi] = bounds;
    let mut out = Vec::new();
    for axis in 0..3 {
        let side = hi[axis] - lo[axis];
        let room = (bed.size[axis] - margin).max(1e-9);
        if side <= room + 1e-9 {
            continue;
        }
        let pieces = (side / room).ceil() as usize;
        for k in 1..pieces {
            out.push(Cut { axis, at: lo[axis] + side * k as f64 / pieces as f64 });
        }
    }
    out
}

/// The cuts that bring `body` within `bed` less `margin`, read off its mesh.
pub fn cuts_of(doc: &DocEngine, body: Id, bed: &Bed, margin: f64) -> Result<Vec<Cut>, DocError> {
    let project = doc.project();
    let b = project.mesh_index(body).and_then(|i| project.bodies[i].mesh.bounds()).ok_or_else(|| DocError::Refused("split-no-body".into()))?;
    Ok(cuts_to_fit([[b.min.x, b.min.y, b.min.z], [b.max.x, b.max.y, b.max.z]], bed, margin))
}

/// The world plane a cut runs in, as a split takes it: its code and its offset along its normal.
fn plane_of(c: &Cut) -> u8 {
    match c.axis {
        0 => 2, // YZ, its normal X
        1 => 1, // XZ, its normal Y
        _ => 0, // XY, its normal Z
    }
}

/// What a cut to fit left: the pieces, and the cuts that made them.
#[derive(Clone, Debug, PartialEq)]
pub struct Split {
    pub pieces: Vec<Id>,
    pub cuts: Vec<Cut>,
}

/// THE PIECES OF `body` CUT TO FIT `bed` less `margin`, as one step; `body` alone when it fits and nothing is cut.
pub fn split_to_fit(doc: &mut DocEngine, body: Id, bed: &Bed, margin: f64) -> Result<Vec<Id>, DocError> {
    // nothing to cut is no step: a batch opened and closed empty would leave one
    if cuts_of(doc, body, bed, margin)?.is_empty() {
        return Ok(vec![body]);
    }
    doc.begin_batch("f-operation");
    match split_in_batch(doc, body, bed, margin) {
        Ok(s) => {
            doc.commit_batch();
            Ok(s.pieces)
        }
        Err(e) => {
            doc.abort_batch();
            Err(e)
        }
    }
}

/// The same within a batch the caller holds open, with the cuts: the joints laid after it go into the same step.
pub fn split_in_batch(doc: &mut DocEngine, body: Id, bed: &Bed, margin: f64) -> Result<Split, DocError> {
    let cuts = cuts_of(doc, body, bed, margin)?;
    if cuts.is_empty() {
        return Ok(Split { pieces: vec![body], cuts });
    }
    if doc.shape(body).is_none() {
        return Err(DocError::Refused("split-needs-faces".into()));
    }
    let mut pieces = vec![body];
    for cut in &cuts {
        let mut next = Vec::new();
        for &piece in &pieces {
            let mut normal = [0.0; 3];
            normal[cut.axis] = 1.0;
            let mut origin = [0.0; 3];
            origin[cut.axis] = cut.at;
            let count = doc.shape(piece).and_then(|s| qymcad_kernel::ask::split_pieces(s, qymcad_core::feature::PlaneAt { origin, normal })).unwrap_or(0);
            if count < 2 {
                next.push(piece);
                continue;
            }
            next.extend(doc.edit("f-operation", |p| Ok(p.add_split_body(piece, plane_of(cut), 0, cut.at, count)))?);
        }
        pieces = next;
    }
    Ok(Split { pieces, cuts })
}
