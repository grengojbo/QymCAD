//! THE PIECES A LOOP OF A DRAWING IS MADE OF, handed from what finds them - a section of a mesh fitted - to what lays
//! them into a sketch.

use super::*;
use crate::feature::Winding;

/// A PIECE OF A LOOP, in the coordinates of the plane it lies in.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    Line {
        from: Point2,
        to: Point2,
    },
    Arc {
        center: Point2,
        radius: f64,
        from: Point2,
        to: Point2,
        winding: Winding,
    },
    /// The whole loop on one circle.
    Circle {
        center: Point2,
        radius: f64,
    },
    /// Through its points; closed when the first and the last are one.
    Spline(Vec<Point2>),
}

impl Piece {
    /// Where the piece begins; a circle has no beginning.
    pub fn start(&self) -> Option<Point2> {
        match self {
            Piece::Line { from, .. } | Piece::Arc { from, .. } => Some(*from),
            Piece::Spline(p) => p.first().copied(),
            Piece::Circle { .. } => None,
        }
    }

    /// Where the piece ends; a circle has no end.
    pub fn end(&self) -> Option<Point2> {
        match self {
            Piece::Line { to, .. } | Piece::Arc { to, .. } => Some(*to),
            Piece::Spline(p) => p.last().copied(),
            Piece::Circle { .. } => None,
        }
    }

    /// The kind of the piece, in a word, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Piece::Line { .. } => "line",
            Piece::Arc { .. } => "arc",
            Piece::Circle { .. } => "circle",
            Piece::Spline(_) => "spline",
        }
    }
}

/// How two pieces meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    /// At an angle.
    Corner,
    /// Along one tangent.
    Smooth,
}

/// A LOOP FITTED: its pieces in order round the loop, each ending where the next begins, and how each meets the next -
/// `joins[k]` is the join of piece `k` and piece `k + 1`, the last of the last and the first.
#[derive(Clone, Debug, Default)]
pub struct Fitted {
    pub pieces: Vec<Piece>,
    pub joins: Vec<Join>,
}
