//! WHAT IS TOO SMALL FOR THE NOZZLE: a round face of a radius under the nozzle's - a hole that closes up as the lines
//! round it run together, a pin thinner than one line - and an edge shorter than the nozzle, a step the line cannot
//! turn on. Read on the exact faces and edges of a body of faces; a mesh brought in has none to read.

use crate::print::checks::{Verdict, PLACES};
use qymcad_core::model::{Id, Project};
use qymcad_kernel::Shape;

/// Which way a round face turns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Round {
    /// Its face looks to its axis: a hole, a rounding of an inner corner.
    Hole,
    /// Its face looks away from its axis: a pin, a rounding of an outer corner.
    Pin,
}

/// A round face too small, in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmallRound {
    pub at: [f64; 3],
    pub radius: f64,
    pub kind: Round,
    /// The face's key, as list_faces gives it.
    pub face: u32,
}

/// An edge too short, in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShortEdge {
    /// Its middle.
    pub at: [f64; 3],
    pub length: f64,
}

/// What is too small for the nozzle, each the smallest first, at most `PLACES` of each.
#[derive(Clone, Debug, PartialEq)]
pub struct Small {
    pub verdict: Verdict,
    pub rounds: Vec<SmallRound>,
    pub edges: Vec<ShortEdge>,
}

/// What of `body`, its exact faces in `shape`, is too small for a nozzle of `nozzle` mm.
pub fn small_features(project: &Project, shape: &Shape, body: Id, nozzle: f64) -> Small {
    let wt = project.body_world_transform(body);
    let place = |p: [f64; 3]| qymcad_core::feature::apply12(&wt, p);
    let mi = project.mesh_index(body);
    let mesh = mi.map(|i| &project.bodies[i].mesh);
    let faces = project.regen_faces.get(&body).map(Vec::as_slice).or_else(|| mi.map(|i| project.bodies[i].faces.as_slice())).unwrap_or(&[]);
    let mut rounds: Vec<SmallRound> = Vec::new();
    for f in faces {
        let Some((o, axis, radius)) = shape.face_cylinder(f.id).filter(|c| c.2 < nozzle) else { continue };
        // the way the face looks against the way out from its axis, on its first triangle
        let kind = match mesh.zip(f.triangles.first()) {
            Some((m, &t)) => {
                let (n, _, c) = m.tri_normal_area(t as usize);
                let off = [c.x - o[0], c.y - o[1], c.z - o[2]];
                let along = off[0] * axis[0] + off[1] * axis[1] + off[2] * axis[2];
                let out = [off[0] - along * axis[0], off[1] - along * axis[1], off[2] - along * axis[2]];
                if out[0] * n[0] + out[1] * n[1] + out[2] * n[2] >= 0.0 {
                    Round::Pin
                } else {
                    Round::Hole
                }
            }
            None => Round::Pin,
        };
        rounds.push(SmallRound { at: place([f.centroid.x, f.centroid.y, f.centroid.z]), radius, kind, face: f.id });
    }
    let mut edges: Vec<ShortEdge> = Vec::new();
    for e in shape.edges_info().into_iter().filter(|e| !e.smooth && e.poly.len() > 1) {
        let p: Vec<[f64; 3]> = e.poly.iter().map(|q| [f64::from(q[0]), f64::from(q[1]), f64::from(q[2])]).collect();
        let length: f64 = p.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2) + (w[1][2] - w[0][2]).powi(2)).sqrt()).sum();
        if length < nozzle && length > 0.0 {
            let ends = [p[0], p[p.len() - 1]];
            edges.push(ShortEdge { at: place([(ends[0][0] + ends[1][0]) / 2.0, (ends[0][1] + ends[1][1]) / 2.0, (ends[0][2] + ends[1][2]) / 2.0]), length });
        }
    }
    rounds.sort_by(|a, b| a.radius.total_cmp(&b.radius));
    edges.sort_by(|a, b| a.length.total_cmp(&b.length));
    let verdict = if rounds.is_empty() && edges.is_empty() { Verdict::Pass } else { Verdict::Warn };
    rounds.truncate(PLACES);
    edges.truncate(PLACES);
    Small { verdict, rounds, edges }
}
