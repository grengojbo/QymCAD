//! THE SMALL RULES OF A FEATURE that every caller laying one needs: which way a cut goes, and where on a face a hole
//! stands. The window and the protocol server both lay cuts and holes; one copy of each rule leaves nothing to drift
//! between what a person gets by clicking and what a caller gets by asking.

use qymcad_core::feature::{PlaneFrame, SketchPlane};

/// A CUT FROM A SKETCH ON A FACE GOES INTO THE BODY - along the negative normal of that face. Taken forward it would
/// cut the void outside and leave the body as it was. A sketch on a world plane or a datum has no body behind it, so
/// its cut goes forward.
pub fn cut_goes_into_the_body(plane: &SketchPlane) -> bool {
    matches!(plane, SketchPlane::Face(..))
}

/// How far a point stands from a face centre along the face's own two axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceOffset {
    pub u: f64,
    pub v: f64,
}

/// A FLAT FACE AS A FRAME: its centre and its normal. The two axes are those of a plane through the centre square to
/// the normal (`PlaneFrame::from_origin_normal`, unturned), so the same face gives the same axes in the window and in
/// every other caller, and a hole typed as two shifts lands where the person put it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceFrame {
    pub centre: [f64; 3],
    pub normal: [f64; 3],
}

impl FaceFrame {
    fn axes(&self) -> PlaneFrame {
        PlaneFrame::from_origin_normal(self.centre, self.normal, 0.0)
    }

    /// How far `at` stands from the centre along the face's axes. A point off the face is taken by its projection.
    pub fn offset_of(&self, at: [f64; 3]) -> FaceOffset {
        let f = self.axes();
        let c = self.centre;
        let d = [at[0] - c[0], at[1] - c[1], at[2] - c[2]];
        let dot = |a: [f64; 3]| a[0] * d[0] + a[1] * d[1] + a[2] * d[2];
        FaceOffset { u: dot(f.x), v: dot(f.y) }
    }

    /// The point `off` from the centre along the face's axes - the centre of a hole given as two shifts.
    pub fn point_at(&self, off: FaceOffset) -> [f64; 3] {
        let f = self.axes();
        let c = self.centre;
        let FaceOffset { u, v } = off;
        [c[0] + f.x[0] * u + f.y[0] * v, c[1] + f.x[1] * u + f.y[1] * v, c[2] + f.x[2] * u + f.y[2] * v]
    }
}

#[cfg(test)]
mod tests {
    use super::{cut_goes_into_the_body, FaceFrame, FaceOffset};
    use qymcad_core::feature::{BasePlane, FaceKey, SketchPlane};

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12)
    }

    /// A CUT GOES INTO THE BODY only from a sketch on a face; from a world plane it goes forward.
    #[test]
    fn a_cut_goes_into_the_body_only_from_a_face() {
        let face = SketchPlane::Face(3, FaceKey { index: 0, centroid: [0.0; 3], normal: [0.0, 0.0, 1.0], id: 0 });
        assert!(cut_goes_into_the_body(&face));
        assert!(!cut_goes_into_the_body(&SketchPlane::World(BasePlane::XY)));
        assert!(!cut_goes_into_the_body(&SketchPlane::Datum(5)));
    }

    /// THE TOP OF A BLOCK has the world axes: a hole 5 along X and 2 along Y stands at centre + (5, 2, 0).
    #[test]
    fn a_face_square_to_z_has_the_world_axes() {
        let top = FaceFrame { centre: [10.0, 20.0, 30.0], normal: [0.0, 0.0, 1.0] };
        assert!(close(top.point_at(FaceOffset { u: 5.0, v: 2.0 }), [15.0, 22.0, 30.0]));
        assert_eq!(top.offset_of(top.centre), FaceOffset { u: 0.0, v: 0.0 });
    }

    /// A POINT ON A SLANTED FACE goes to two shifts and back to the same point; a point off the face comes back as
    /// its projection.
    #[test]
    fn shifts_and_back_give_the_same_point() {
        let n = 3f64.sqrt().recip();
        let face = FaceFrame { centre: [1.0, -2.0, 4.0], normal: [n, n, n] };
        let off = FaceOffset { u: 7.5, v: -3.25 };
        let at = face.point_at(off);
        let back = face.offset_of(at);
        assert!((back.u - off.u).abs() < 1e-12 && (back.v - off.v).abs() < 1e-12, "{back:?}");
        let lifted = [at[0] + 2.0 * n, at[1] + 2.0 * n, at[2] + 2.0 * n];
        assert!(close(face.point_at(face.offset_of(lifted)), at));
    }
}
