//! Parametric backbone: the data model of the feature timeline.
//!
//! Sketches, datum planes and part features are nodes of one ordered timeline (`Project::timeline`).
//! References use stable ids only, never indices. The 2D geometry of a sketch stays 2D; the plane is only a
//! placement, and the plane frame lifts a 2D point into the world (`PlaneFrame::lift`).

use serde::{Deserialize, Serialize};

use crate::geom::{Built, Mesh, MeshFace, Point2, Point3};
use crate::model::Id;

/// Base plane of the global coordinate system, used to place a sketch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum BasePlane {
    #[default]
    XY,
    XZ,
    YZ,
}

/// Stable key of a planar face of a body (topological naming).
///
/// It carries the positional index of the face at the time the body was created, as a fast path, plus a
/// geometric fingerprint (centre and normal) as a fallback for when the index shifts after a rebuild.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FaceKey {
    pub index: u32,
    pub centroid: [f64; 3],
    pub normal: [f64; 3],
    /// Persistent face id from the B-rep. It survives a rebuild, so the reference does not drift. Zero means
    /// unknown (an older key, or a mesh). Resolution goes by id, with the centroid and normal fingerprint as
    /// the fallback match.
    #[serde(default)]
    pub id: u32,
}

/// Which face of a size between two faces moves when the size is changed: the first picked, the second, or both by
/// half, the middle standing still.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapMoves {
    #[default]
    One,
    Other,
    Both,
}

/// How far each face of a size between faces moves along its own normal, out of the body for more.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GapShift {
    pub one: f64,
    pub other: f64,
}

impl GapMoves {
    /// The moves that make two parallel flat faces `gap` apart: `apart` is how far the second stands from the first along
    /// the normal of the first, `turn` the cosine between their normals (1 or -1). The first face moved by t along its
    /// own normal leaves them `apart - t`; the second, `apart + t * turn`.
    pub fn shifts(self, apart: f64, turn: f64, gap: f64) -> GapShift {
        let want = apart.signum() * gap;
        match self {
            GapMoves::One => GapShift { one: apart - want, other: 0.0 },
            GapMoves::Other => GapShift { one: 0.0, other: (want - apart) / turn },
            GapMoves::Both => GapShift { one: (apart - want) / 2.0, other: (want - apart) / 2.0 / turn },
        }
    }
}

/// Placement of a sketch in 3D. The 2D geometry is lifted into the world through the plane frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SketchPlane {
    /// A base plane of the global coordinate system (XY, XZ, YZ).
    World(BasePlane),
    /// A datum plane feature, by id (in `Project::planes`).
    Datum(Id),
    /// A planar face of a body: the body id plus the face key.
    Face(Id, FaceKey),
}

impl Default for SketchPlane {
    fn default() -> Self {
        SketchPlane::World(BasePlane::XY)
    }
}

mod frames;
pub use frames::*;
mod mates;
pub use mates::*;
mod kind;
pub use kind::*;
mod matrix;
pub use matrix::*;
mod op_types;
pub use op_types::*;
mod kernel;
pub use kernel::*;
