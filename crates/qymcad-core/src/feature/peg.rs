//! JOINTS ACROSS A CUT: what holds two pieces a cut left together again - a peg on one standing in a socket of the
//! other, or a pin standing in a socket of each. The sizes are the node's, kept as numbers a formula can drive: the
//! diameter, how deep the peg goes, the gap between peg and socket, and the taper of a round peg.
//!
//! A peg is tapered so it prints standing on the cut without supports and finds its socket; the socket is the peg
//! grown by the gap all round and deeper by the gap, so the peg bears on its sides and never bottoms out.

use super::*;
use crate::geom::{Contour, Point2, ProfEdge};

/// What holds the pieces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PegKind {
    /// A peg grown from the piece below the cut, a socket in the piece above.
    Plug,
    /// A socket in each piece and a pin of its own between them, printed apart.
    Dowel,
}

/// The cross-section of a peg.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PegShape {
    Round,
    /// Six sides: it cannot turn in its socket, so one peg holds a piece against turning.
    Hex,
}

/// THE JOINTS OF A CUT: their kind and shape, and their sizes in mm - the taper in degrees off the axis.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PegSpec {
    pub kind: PegKind,
    pub shape: PegShape,
    pub diameter: f64,
    pub depth: f64,
    pub clearance: f64,
    #[serde(default)]
    pub taper: f64,
}

/// How far a peg reaches back into the piece it grows from, mm: past the cut, so the union holds a solid neck rather
/// than two faces touching.
pub const PEG_ROOT: f64 = 0.05;

/// Where a joint stands: the point on the cut and the way from it the tool runs, a unit vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stand {
    pub at: [f64; 3],
    pub along: [f64; 3],
}

/// A tool cut into a piece, or grown from it: where it stands, which way it runs, how wide at its root and at its end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PegTool {
    /// The point on the cut it stands at.
    pub at: [f64; 3],
    /// The way it runs from the cut, a unit vector.
    pub along: [f64; 3],
    /// From where to where along `along`, mm from the cut.
    pub from: f64,
    pub to: f64,
    /// The radius at `from` and at `to`; the circle through the corners for a hex.
    pub root: f64,
    pub end: f64,
}

impl PegSpec {
    /// The tan of the taper: how much the radius closes a mm along the peg.
    fn closing(&self) -> f64 {
        self.taper.clamp(0.0, 30.0).to_radians().tan()
    }

    /// The peg, grown from the cut along `along`: from just under the cut to `depth`, narrowing by the taper.
    pub fn peg(&self, stand: Stand) -> PegTool {
        let Stand { at, along } = stand;
        let r = self.diameter / 2.0;
        PegTool { at, along, from: -PEG_ROOT, to: self.depth, root: r + PEG_ROOT * self.closing(), end: r - self.depth * self.closing() }
    }

    /// The socket the peg stands in, cut along `along`: the peg grown by the clearance all round and deeper by it.
    pub fn socket(&self, stand: Stand) -> PegTool {
        let Stand { at, along } = stand;
        let r = self.diameter / 2.0 + self.clearance;
        let deep = self.depth + self.clearance;
        PegTool { at, along, from: -PEG_ROOT, to: deep, root: r + PEG_ROOT * self.closing(), end: r - deep * self.closing() }
    }

    /// The pin of a dowel: the peg each way across the cut, the clearance off its ends.
    pub fn pin(&self, stand: Stand) -> PegTool {
        let Stand { at, along } = stand;
        let r = self.diameter / 2.0;
        let reach = self.depth - self.clearance;
        PegTool { at, along, from: -reach, to: reach, root: r, end: r }
    }
}

impl PegTool {
    /// Where the tool's profile is drawn: its X across the tool, its Y along it, at the point on the cut.
    pub fn place(&self) -> [f64; 12] {
        let n = self.along;
        let seed = if n[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
        let dot = seed[0] * n[0] + seed[1] * n[1] + seed[2] * n[2];
        let u = [seed[0] - dot * n[0], seed[1] - dot * n[1], seed[2] - dot * n[2]];
        let l = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt().max(1e-300);
        let u = [u[0] / l, u[1] / l, u[2] / l];
        let w = [u[1] * n[2] - u[2] * n[1], u[2] * n[0] - u[0] * n[2], u[0] * n[1] - u[1] * n[0]];
        [u[0], n[0], w[0], self.at[0], u[1], n[1], w[1], self.at[1], u[2], n[2], w[2], self.at[2]]
    }

    /// A round tool's half-section, to be turned about the profile's Y: from the axis out to the radius at `from`, up
    /// to the radius at `to`, back to the axis.
    pub fn half_section(&self) -> Contour {
        let p = Point2::new;
        Contour::closed(vec![p(0.0, self.from), p(self.root, self.from), p(self.end, self.to), p(0.0, self.to)])
    }

    /// A hex tool's section across it, the plane its profile is drawn in turned to stand across the tool: six corners
    /// on the circle of `root`. A hex is not tapered: six sides already find their socket.
    pub fn hex_section(&self) -> Contour {
        let corners: Vec<Point2> = (0..6).map(|k| std::f64::consts::TAU * f64::from(k) / 6.0).map(|a| Point2::new(self.root * a.cos(), self.root * a.sin())).collect();
        let edges = (0..6).map(|k| ProfEdge::Line { a: corners[k], b: corners[(k + 1) % 6] }).collect();
        Contour { points: corners, closed: true, edges, edge_src: Vec::new() }
    }

    /// Where a hex tool's section is drawn: its plane across the tool, raised to `from`, its normal along it.
    pub fn hex_place(&self) -> [f64; 12] {
        let m = self.place();
        // the profile's X stays across, its Y becomes the third axis across, its normal (Z) the tool's way
        let (u, n, w) = ([m[0], m[4], m[8]], [m[1], m[5], m[9]], [m[2], m[6], m[10]]);
        let o = [self.at[0] + n[0] * self.from, self.at[1] + n[1] * self.from, self.at[2] + n[2] * self.from];
        [u[0], -w[0], n[0], o[0], u[1], -w[1], n[1], o[1], u[2], -w[2], n[2], o[2]]
    }
}
