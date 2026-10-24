//! THE EDGES OF THE BODIES AS LINES: a pair of points in the world for every sharp edge of a body (`Caches::sharp`),
//! the part of it a section keeps. The card draws them and the raster draws them, from this one list.

use crate::{section_eff, BodyEdges, GpuVert, Painting};

/// THE SHARP EDGES OF ONE BODY: each a pair of its mesh vertices, shared with every frame that draws them.
#[derive(Clone, Default)]
pub struct EdgeList {
    pub ends: std::sync::Arc<Vec<[u32; 2]>>,
}

/// Whose edges to lay as lines: the body by its mesh index, where it stands in the world, and its row of the look table.
#[derive(Clone, Copy)]
pub struct EdgesOf {
    pub mi: usize,
    pub world: [f64; 12],
    pub row: u32,
}

/// THE EDGES OF A BODY AS LINES FOR THE CARD: a pair of vertices for each sharp edge (`Caches::sharp`), in the world,
/// naming the body's row; none when the edges are hidden. A section keeps the part of an edge on the kept side.
pub fn edge_lines(pn: &Painting, of: EdgesOf) -> Vec<GpuVert> {
    if pn.set.body_edges != BodyEdges::Shown {
        return Vec::new();
    }
    let sharp = pn.cache.sharp.borrow();
    let Some(edges) = sharp.value.get(of.mi) else { return Vec::new() };
    let Some(body) = pn.project.bodies.get(of.mi) else { return Vec::new() };
    let ident = qymcad_core::feature::is_identity12(&of.world);
    let world = |vi: u32| {
        let p = body.mesh.verts[vi as usize];
        if ident {
            [p.x, p.y, p.z]
        } else {
            qymcad_core::feature::apply12(&of.world, [p.x, p.y, p.z])
        }
    };
    let cut = section_eff(&pn.cut()).map(|plane| CutPlane { o: plane.0, n: plane.1 });
    let mut out = Vec::with_capacity(edges.ends.len() * 2);
    for e in edges.ends.iter() {
        let Some(seg) = kept_by(cut, [world(e[0]), world(e[1])]) else { continue };
        for p in seg {
            out.push(GpuVert { pos: [p[0] as f32, p[1] as f32, p[2] as f32], body: of.row, nrm: 0 });
        }
    }
    out
}

/// The plane of a section: a point on it and its normal, the side it cuts away along the normal.
#[derive(Clone, Copy)]
pub struct CutPlane {
    pub o: [f64; 3],
    pub n: [f64; 3],
}

/// The part of the segment `seg` a section keeps - the side the bodies keep, d <= 0 - or all of it with none.
pub fn kept_by(cut: Option<CutPlane>, seg: [[f64; 3]; 2]) -> Option<[[f64; 3]; 2]> {
    let Some(CutPlane { o, n }) = cut else { return Some(seg) };
    let d = |p: [f64; 3]| (p[0] - o[0]) * n[0] + (p[1] - o[1]) * n[1] + (p[2] - o[2]) * n[2];
    let (da, db) = (d(seg[0]), d(seg[1]));
    if da > 0.0 && db > 0.0 {
        return None;
    }
    if da <= 0.0 && db <= 0.0 {
        return Some(seg);
    }
    let t = da / (da - db);
    let at = [seg[0][0] + (seg[1][0] - seg[0][0]) * t, seg[0][1] + (seg[1][1] - seg[0][1]) * t, seg[0][2] + (seg[1][2] - seg[0][2]) * t];
    Some(if da <= 0.0 { [seg[0], at] } else { [at, seg[1]] })
}
