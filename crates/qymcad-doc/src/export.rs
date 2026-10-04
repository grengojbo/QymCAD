//! A FILE WRITTEN OUT OF THE DOCUMENT: which bodies go, as what tree, and the writing itself - a mesh at a detail, or
//! the exact solids.
//!
//! The window sorts the bodies on its own thread and writes in a thread of its own, with the live shapes moved there
//! and back; a headless caller writes here and now from shapes it lends. Both write through the same functions over
//! borrowed shapes, so the file is the same whoever asked for it. The status line and the file chooser stay in the
//! window.
use crate::import::MeshFormat;
use qymcad_core::geom::{Built, Mesh, MeshFace};
use qymcad_core::model::{ExportKind, ExportMesh, ExportNode, Id, Project};
use qymcad_kernel::Shape;
use std::collections::HashMap;

/// What to write in 3D: a component with every body nested in its subtree, or the whole project. In both cases ONLY
/// the visible bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportTarget {
    Component(Id),
    Project,
}

/// Whether `owner` and every assembly above it, up to `stop`, is shown.
pub fn component_chain_visible(project: &Project, owner: Id, stop: Option<Id>) -> bool {
    let mut cur = Some(owner);
    while let Some(id) = cur {
        if Some(id) == stop {
            break;
        }
        let Some(c) = project.components.iter().find(|c| c.id == id) else {
            break;
        };
        if !c.visible {
            return false;
        }
        cur = c.parent;
    }
    true
}

/// THE BODIES THAT GO OUT for `target`: the visible ones, in a visible chain of components.
///
/// CRITICAL: the CONSUMED bodies are excluded (the bases eaten by cuts, booleans and modifiers). In 3D they are hidden
/// by a separate filter, `consumed_bodies`, not by `visible`, so the export used to pull them in TOGETHER with the
/// final body - the bases covered the cuts, and the STEP or STL came out as a solid blank with nothing cut away.
pub fn visible_bodies(project: &Project, target: ExportTarget) -> Vec<Id> {
    let subtree: Option<std::collections::HashSet<Id>> = match target {
        ExportTarget::Project => None,
        ExportTarget::Component(cid) => {
            let mut s: std::collections::HashSet<Id> = project.descendants(cid).into_iter().collect();
            s.insert(cid);
            Some(s)
        }
    };
    let consumed = project.consumed_bodies();
    project
        .bodies
        .iter()
        .filter(|b| b.visible)
        .map(|b| b.id)
        .filter(|b| !consumed.contains(b))
        .filter(|&b| project.body_owner(b).is_none_or(|o| component_chain_visible(project, o, None)))
        .filter(|&b| match &subtree {
            None => true,
            Some(s) => project.body_owner(b).is_some_and(|o| s.contains(&o)),
        })
        .collect()
}

/// THE BODIES SORTED BY WHAT CAN GO INTO A FILE. STEP and STL take the same sort, so the contents of the two files do
/// not drift apart SILENTLY.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// A live B-rep - the exact geometry.
    pub brep: Vec<Id>,
    /// An STL import: there never was a B-rep.
    pub mesh_only: Vec<Id>,
    /// A failed rebuild: the last good geometry is on the screen and there is no B-rep.
    pub stale: Vec<Id>,
}

impl Plan {
    /// The bodies a mesh file takes: everything visible (a B-rep is re-tessellated, the rest go as their stored mesh).
    pub fn mesh_bodies(&self) -> Vec<Id> {
        self.brep.iter().chain(&self.mesh_only).chain(&self.stale).copied().collect()
    }
}

/// Sort the bodies that go out for `target` by whether `has_shape` holds a live B-rep for them.
pub fn plan(project: &Project, target: ExportTarget, has_shape: impl Fn(Id) -> bool) -> Plan {
    let mut plan = Plan::default();
    for b in visible_bodies(project, target) {
        match project.export_kind(b, has_shape(b)) {
            ExportKind::Brep => plan.brep.push(b),
            ExportKind::MeshOnly => plan.mesh_only.push(b),
            ExportKind::Stale => plan.stale.push(b),
        }
    }
    plan
}

/// The tree under `target` as a file takes it, with the names a person sees (`shown`); `bodies` are the ones that go
/// out.
pub fn tree_to_write(project: &Project, target: ExportTarget, bodies: &[Id], shown: &dyn Fn(&str) -> String) -> Vec<ExportNode> {
    let root = match target {
        ExportTarget::Project => project.root,
        ExportTarget::Component(c) => c,
    };
    let mut tree = project.export_tree(root, |b| bodies.contains(&b));
    for n in &mut tree {
        n.name = shown(&n.name); // the name the tree shows, not a catalogue key
    }
    tree
}

/// THE TREE A MESH FILE GOES OUT AS, where the format holds one: glTF keeps the parts as nodes under their names, 3MF
/// as an object of parts under theirs, placed, in their colours. The rest go out flat - an empty tree.
pub fn mesh_tree(project: &Project, format: MeshFormat, target: ExportTarget, bodies: &[Id], shown: &dyn Fn(&str) -> String) -> Vec<ExportNode> {
    if matches!(format, MeshFormat::Glb | MeshFormat::ThreeMf) {
        tree_to_write(project, target, bodies, shown)
    } else {
        Vec::new()
    }
}

/// THE TREE AN EXACT FILE GOES OUT AS. STEP carries the assembly: its subassemblies and parts under the names the tree
/// shows, their colours, every component in its place, a clone as a second occurrence of its original. IGES has no
/// tree, and goes out flat - an empty tree. `bodies` are the ones that go out - visible, with a live B-rep.
pub fn exact_tree(project: &Project, format: qymcad_kernel::ExactFormat, target: ExportTarget, bodies: &[Id], shown: &dyn Fn(&str) -> String) -> Vec<ExportNode> {
    if format != qymcad_kernel::ExactFormat::Step {
        return Vec::new();
    }
    tree_to_write(project, target, bodies, shown)
}

/// A body's mesh in its own coordinates and where it stands in the world.
pub struct Placed {
    pub own: ExportMesh,
    pub place: [f64; 12],
}

/// The stored mesh of a body with no live B-rep, where it stands; a piece of a mesh coloured triangle by triangle keeps
/// its colours on the way out. `None` for a body the document does not hold.
pub fn stored_mesh(project: &Project, body: Id) -> Option<Placed> {
    let i = project.mesh_index(body)?;
    let tri = project
        .tri_colors
        .get(&project.lineage_root(body))
        .filter(|(_, places)| places.len() == project.bodies[i].mesh.tris.len())
        .map(|(palette, places)| places.iter().map(|&k| palette.get(k as usize).copied()).collect())
        .unwrap_or_default();
    Some(Placed { own: ExportMesh { body, mesh: project.bodies[i].mesh.clone(), tri_colors: tri }, place: project.body_world_transform(body) })
}

/// What a mesh file is written as: its format, the detail a live B-rep is tessellated to (deflection in mm), and the
/// tree (empty to go out flat).
pub struct MeshOut {
    pub format: MeshFormat,
    pub deflection: f64,
    pub tree: Vec<ExportNode>,
}

/// What went into a file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Written {
    /// The bodies written.
    pub bodies: usize,
    /// The live bodies whose tessellation failed: NOT dropped silently but counted.
    pub failed: usize,
}

/// WRITE A MESH FILE: every live solid tessellated at the detail, every stored mesh as it is. A tree goes out with
/// every part in its own coordinates, placed by the tree; flat, every body where it stands in the world. The error is
/// the writer's code.
pub fn write_mesh(out: &MeshOut, solids: &[(Id, &Shape, [f64; 12])], stored: Vec<Placed>, path: &str) -> Result<Written, String> {
    let mut own: Vec<Placed> = Vec::with_capacity(solids.len() + stored.len());
    let mut failed = 0usize;
    for (id, s, m) in solids {
        if let Some(Built { mesh, faces }) = s.tessellate_merged(out.deflection) {
            let tri = tri_colours(&out.tree, *id, &mesh, &faces);
            own.push(Placed { own: ExportMesh { body: *id, mesh, tri_colors: tri }, place: *m });
        } else {
            failed += 1;
        }
    }
    own.extend(stored);
    let n = own.len();
    if out.tree.is_empty() {
        let world: Vec<Mesh> = own
            .into_iter()
            .map(|Placed { own: ExportMesh { mut mesh, .. }, place }| {
                mesh.transform(&place);
                mesh
            })
            .collect();
        write_meshes(out.format, &world, path)?;
    } else {
        let own: Vec<ExportMesh> = own.into_iter().map(|p| p.own).collect();
        match out.format {
            MeshFormat::ThreeMf => qymcad_io::export_3mf_tree(&out.tree, &own, path)?,
            _ => qymcad_io::export_glb_tree(&out.tree, &own, path)?, // a tree is given to GLB and 3MF alone
        }
    }
    Ok(Written { bodies: n, failed })
}

/// WRITE AN EXACT FILE: a STEP goes out as the document's tree, a format with none flat, every solid where it stands.
/// The error is the writer's code.
pub fn write_exact(format: qymcad_kernel::ExactFormat, tree: &[ExportNode], solids: &[(Id, &Shape, [f64; 12])], path: &str) -> Result<Written, String> {
    if tree.is_empty() {
        let pairs: Vec<(&Shape, [f64; 12])> = solids.iter().map(|(_, s, m)| (*s, *m)).collect();
        qymcad_kernel::write_exact(format, &pairs, path)?;
    } else {
        let by_id: Vec<(Id, &Shape)> = solids.iter().map(|(id, s, _)| (*id, *s)).collect();
        qymcad_kernel::write_step_tree(tree, &by_id, path)?;
    }
    Ok(Written { bodies: solids.len(), failed: 0 })
}

/// The live solids of `bodies` where they stand in the world; a body with no live shape is passed over.
pub fn placed_solids<'a>(project: &Project, shapes: &'a HashMap<Id, Shape>, bodies: &[Id]) -> Vec<(Id, &'a Shape, [f64; 12])> {
    bodies.iter().filter_map(|&b| shapes.get(&b).map(|s| (b, s, project.body_world_transform(b)))).collect()
}

/// Write meshes in the format asked for.
fn write_meshes(format: MeshFormat, meshes: &[Mesh], path: &str) -> Result<(), String> {
    match format {
        MeshFormat::Stl => qymcad_io::export_stl(meshes, path),
        MeshFormat::Obj => qymcad_io::export_obj(meshes, path),
        MeshFormat::Ply => qymcad_io::export_ply(meshes, path),
        MeshFormat::Glb => qymcad_io::export_glb(meshes, path),
        MeshFormat::ThreeMf => qymcad_io::export_3mf(meshes, path),
        MeshFormat::Amf => qymcad_io::export_amf(meshes, path),
    }
}

/// The colour, or none, of every triangle of `body`'s tessellation where the tree gives faces of it a colour of their
/// own: the body's colour (none for a part in the palette, which goes out with none), and each coloured face's over it,
/// by the face's persistent id. Empty where no face has one.
fn tri_colours(tree: &[ExportNode], body: Id, mesh: &Mesh, faces: &[MeshFace]) -> Vec<Option<[u8; 3]>> {
    let Some(node) = tree.iter().find(|n| n.body == Some(body)).filter(|n| !n.face_colors.is_empty()) else { return Vec::new() };
    let mut out = vec![node.color; mesh.tris.len()];
    for f in faces.iter().filter(|f| f.id != 0) {
        if let Some((_, c)) = node.face_colors.iter().find(|(id, _)| *id == f.id) {
            for &t in &f.triangles {
                if let Some(slot) = out.get_mut(t as usize) {
                    *slot = Some(*c);
                }
            }
        }
    }
    out
}
