//! THE EDGES OF THE BODIES AS LINES: a pair of points in the world for every sharp edge of a body (`Caches::sharp`),
//! the part of it a section keeps. The card draws them and the raster draws them, from this one list. And the colour a
//! feature found on a body is shown in.

use crate::{section_eff, BodyEdges, GpuVert, MeshLines, Painting};

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

/// THE EDGES OF A BODY AS LINES FOR THE CARD: a pair of vertices for each sharp edge (`Caches::sharp`) and, while "Mesh
/// triangles" is on, for each side of a triangle it shows (`Caches::mesh_lines`), in the world, naming the body's row;
/// none when both are hidden. A section keeps the part of an edge on the kept side.
pub fn edge_lines(pn: &Painting, of: EdgesOf) -> Vec<GpuVert> {
    let edges_on = pn.set.body_edges == BodyEdges::Shown;
    let mesh_on = pn.set.mesh_lines == MeshLines::Shown;
    if !edges_on && !mesh_on {
        return Vec::new();
    }
    if mesh_on {
        ensure_mesh_lines(pn.cache, pn.project, pn.regen);
    }
    let take = |c: &crate::Cached<Vec<EdgeList>>| c.value.get(of.mi).map(|e| e.ends.clone());
    let none = || Some(EdgeList::default().ends);
    let Some(sharp_ends) = (if edges_on { take(&pn.cache.sharp.borrow()) } else { none() }) else { return Vec::new() };
    let Some(mesh_ends) = (if mesh_on { take(&pn.cache.mesh_lines.borrow()) } else { none() }) else { return Vec::new() };
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
    let mut out = Vec::with_capacity((sharp_ends.len() + mesh_ends.len()) * 2);
    for e in sharp_ends.iter().chain(mesh_ends.iter()) {
        let Some(seg) = kept_by(cut, [world(e[0]), world(e[1])]) else { continue };
        for p in seg {
            out.push(GpuVert { pos: [p[0] as f32, p[1] as f32, p[2] as f32], body: of.row, nrm: 0 });
        }
    }
    out
}

/// What a body is made as, for the lines "Mesh triangles" draws over it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MadeAs {
    /// A piece of a mesh brought from a file: its triangles are all there is.
    MeshPiece,
    /// A body of faces the kernel built.
    Faces,
}

/// THE SIDES OF TRIANGLES "MESH TRIANGLES" DRAWS OVER A BODY: every triangle of a piece of a mesh; on a body of faces
/// only the faces of one triangle. A recognition lays every triangle of a region no surface fitted as a flat face of its
/// own (`qymcad_kernel::recognise`), while an exact face comes back from the kernel as many - a side of a box as 2, a
/// hole's wall as dozens - so a face of one triangle is where the mesh stayed. A triangular face of a part drawn so
/// shows the three sides it has as edges anyway.
pub fn mesh_lines_of(body: &qymcad_core::model::Body, made: MadeAs) -> Vec<[u32; 2]> {
    match made {
        MadeAs::MeshPiece => body.mesh.sides(&(0..body.mesh.tris.len() as u32).collect::<Vec<u32>>()),
        MadeAs::Faces => body.mesh.sides(&body.faces.iter().filter(|f| f.triangles.len() == 1).flat_map(|f| f.triangles.iter().copied()).collect::<Vec<u32>>()),
    }
}

/// THE LINES OF "MESH TRIANGLES" FOR EVERY BODY, found once per change of the geometry (`Caches::mesh_lines`).
pub fn ensure_mesh_lines(cache: &crate::Caches, project: &qymcad_core::model::Project, regen: &crate::Rebuilding) {
    let mut c = cache.mesh_lines.borrow_mut();
    if c.rev == regen.geom_rev && c.value.len() == project.bodies.len() {
        return;
    }
    let pieces: std::collections::HashSet<qymcad_core::model::Id> = project
        .timeline
        .iter()
        .filter_map(|n| match n.kind {
            qymcad_core::feature::FeatureKind::MeshPiece { body, .. } => Some(body),
            _ => None,
        })
        .collect();
    c.value = project.bodies.iter().map(|b| EdgeList { ends: std::sync::Arc::new(mesh_lines_of(b, if pieces.contains(&b.id) { MadeAs::MeshPiece } else { MadeAs::Faces })) }).collect();
    c.rev = regen.geom_rev;
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

/// THE COLOUR OF A FEATURE FOUND ON A BODY, by its kind: a hole, a chamfer, a rounding. The one table the model is lit
/// by and the list of Recognise all marks its groups with, so a row reads as the faces it lights.
pub fn feature_colour(pal: &qymcad_scheme::Palette, exact: &qymcad_core::model::Exact) -> egui::Color32 {
    match exact {
        qymcad_core::model::Exact::Hole { .. } => pal.remove(),
        qymcad_core::model::Exact::Chamfer { .. } => pal.modify(),
        qymcad_core::model::Exact::Round { .. } => pal.reference(),
    }
}
