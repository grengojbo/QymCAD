//! A BODY READY FOR A PRINTER: what a mesh is made of, whether it prints, how it lies on the bed and how it is cut to
//! fit. Numbers only: what a reader is told, and in which words, is the caller's.

pub mod checks;
pub mod mesh_repair;
pub mod mesh_report;
pub mod thickness;

/// The triangles of `body` placed in the world, as its part places it; `None` for a body the document does not hold.
pub fn world_mesh(project: &qymcad_core::model::Project, body: qymcad_core::model::Id) -> Option<qymcad_core::geom::Mesh> {
    let mut mesh = project.bodies.get(project.mesh_index(body)?)?.mesh.clone();
    let wt = project.body_world_transform(body);
    for v in &mut mesh.verts {
        let w = qymcad_core::feature::apply12(&wt, [v.x, v.y, v.z]);
        *v = qymcad_core::geom::Point3::new(w[0], w[1], w[2]);
    }
    Some(mesh)
}
