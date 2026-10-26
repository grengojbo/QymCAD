//! THE JOINTS OF A CUT BUILT: at every spot a peg grown from the piece behind the cut and a socket cut in the piece in
//! front, or a socket in each and a pin of its own - each one boolean on the piece as it stands after the spot before.

use super::*;
use crate::geom::Built;
use crate::feature::{AxisLine, PegKind, PegShape, PegSpec, PegTool, RevolveAbout, Sides, Stand};

/// How a tool meets the piece: grown from it, or cut into it.
#[derive(Clone, Copy)]
enum Meet {
    Grow,
    Cut,
}

impl Meet {
    /// The kernel's own code for it.
    fn op(self) -> u8 {
        match self {
            Meet::Grow => 1,
            Meet::Cut => 0,
        }
    }
}

/// The pieces of one joint node and the plane between them.
pub(crate) struct JoinedPieces<'a> {
    pub a: Id,
    pub b: Id,
    pub normal: [f64; 3],
    pub spots: &'a [[f64; 3]],
    pub bodies: &'a [Id],
}

impl Project {
    /// Joined: one branch of the timeline rebuild. Returns whether the node's error record may be cleared (see
    /// `apply_regen`).
    pub(crate) fn regen_joined(&mut self, p: &mut Pass, pieces: JoinedPieces, joint: PegSpec) -> bool {
        // the sizes follow their formulas as every feature's numbers do
        let spec =
            PegSpec { diameter: p.dim("diameter", joint.diameter), depth: p.dim("depth", joint.depth), clearance: p.dim("clearance", joint.clearance), taper: p.dim("taper", joint.taper), ..joint };
        let JoinedPieces { a, b, normal, spots, bodies } = pieces;
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        let n = if len > 1e-12 { [normal[0] / len, normal[1] / len, normal[2] / len] } else { [0.0, 0.0, 1.0] };
        let back = [-n[0], -n[1], -n[2]];
        let fails = |e: crate::errors::CoreError| (bodies.to_vec(), e);
        let built = (|| {
            if spec.diameter <= 0.0 || spec.depth <= 0.0 || spec.clearance < 0.0 || spots.is_empty() {
                return Err(fails(crate::errors::CoreError::OpFailed(crate::errors::Op::Boolean)));
            }
            let (out_a, out_b) = (bodies[0], bodies[1]);
            let mut last_a = Ok(Built::default());
            let mut last_b = Ok(Built::default());
            let mut last_pin = None;
            for (k, &at) in spots.iter().enumerate() {
                let src_a = if k == 0 { a } else { out_a };
                let src_b = if k == 0 { b } else { out_b };
                match spec.kind {
                    PegKind::Plug => {
                        last_a = tool_on(p.kernel, BodyOp { src: src_a, op: Meet::Grow.op(), body: out_a }, &spec.peg(Stand { at, along: n }), spec.shape, p.node);
                        last_b = tool_on(p.kernel, BodyOp { src: src_b, op: Meet::Cut.op(), body: out_b }, &spec.socket(Stand { at, along: n }), spec.shape, p.node);
                    }
                    PegKind::Dowel => {
                        last_a = tool_on(p.kernel, BodyOp { src: src_a, op: Meet::Cut.op(), body: out_a }, &spec.socket(Stand { at, along: back }), spec.shape, p.node);
                        last_b = tool_on(p.kernel, BodyOp { src: src_b, op: Meet::Cut.op(), body: out_b }, &spec.socket(Stand { at, along: n }), spec.shape, p.node);
                        if let Some(&pin) = bodies.get(2) {
                            let src = if k == 0 { 0 } else { pin };
                            last_pin = Some(tool_on(p.kernel, BodyOp { src, op: Meet::Grow.op(), body: pin }, &spec.pin(Stand { at, along: n }), spec.shape, p.node));
                        }
                    }
                }
                if last_a.is_err() || last_b.is_err() || last_pin.as_ref().is_some_and(Result::is_err) {
                    break;
                }
            }
            Ok((last_a, last_b, last_pin))
        })();
        let mut clear = true;
        match built {
            Ok((ra, rb, rpin)) => {
                clear &= self.apply_regen(p.landing(), bodies[0], ra);
                clear &= self.apply_regen(p.landing(), bodies[1], rb);
                if let (Some(&pin), Some(r)) = (bodies.get(2), rpin) {
                    clear &= self.apply_regen(p.landing(), pin, r);
                }
            }
            Err((all, e)) => {
                for b in all {
                    clear &= self.apply_regen(p.landing(), b, Err(e.clone()));
                }
            }
        }
        clear
    }
}

/// One tool laid on a piece as `bo` asks: a round one turned from its half-section about its axis, a hex one drawn
/// across and drawn along.
fn tool_on(kernel: &dyn crate::feature::Kernel, bo: BodyOp, tool: &PegTool, shape: PegShape, node: Id) -> Result<Built, crate::errors::CoreError> {
    let sides = Sides { feature: node, sources: &[], regions: &[] };
    match shape {
        PegShape::Round => {
            let profile = crate::geom::encode_profile(&tool.half_section(), &[]);
            let about = RevolveAbout { axis: 1, line: Some(AxisLine { origin: [0.0; 3], dir: [0.0, 1.0, 0.0] }) };
            kernel.revolve_region_multi(bo, &[profile], about, 360.0, tool.place(), sides)
        }
        PegShape::Hex => {
            let profile = crate::geom::encode_profile(&tool.hex_section(), &[]);
            if bo.src == 0 {
                kernel.extrude_region(bo.body, &profile, tool.to - tool.from, tool.hex_place())
            } else {
                kernel.combine_region(bo.body, bo.src, &profile, tool.to - tool.from, bo.op, tool.hex_place())
            }
        }
    }
}
