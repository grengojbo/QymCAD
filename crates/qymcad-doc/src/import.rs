//! A FILE BROUGHT INTO THE DOCUMENT: read, laid in as the file's tree of subassemblies and parts, and taken at a
//! factor.
//!
//! STL, OBJ and PLY carry no unit, and a file that names one can name it wrong. Measured on sample files: a cube came
//! in at 1 mm and a cow at 10 mm, and an IGES bearing and a hammer that both say "millimetres" at 0.1 mm and 38.9 m -
//! exactly as their numbers are. So what is laid in keeps the file's own numbers beside it, and any factor is taken
//! from those, never from a factor taken before.
//!
//! The words of the window - the question asked, the status line, the camera sent to what came in - are not here; the
//! window asks and says, this lays in and scales.
use crate::brep::scale_matrix;
use qymcad_core::geom::{Built, Mesh, MeshFace, Point3};
use qymcad_core::model::{FileGroup, Id, ImportNode, Project};
use qymcad_kernel::Shape;
use std::collections::HashMap;

/// A model whose largest side is under this, or over `LARGEST_MM`, is asked about even when its file names a unit:
/// both ends were met in sample files, and no part a person imports is either size.
pub const SMALLEST_MM: f64 = 1.0;
pub const LARGEST_MM: f64 = 10_000.0;

/// THE MESH FORMATS a body is read from or written as. Their names and extensions live in the table of formats
/// (`qymcad_io::Format`); this says only which one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshFormat {
    Stl,
    Obj,
    Ply,
    Glb,
    ThreeMf,
    Amf,
}

impl MeshFormat {
    /// The entry in the table of formats: the name and the extensions live there, once.
    pub fn entry(self) -> qymcad_io::Format {
        match self {
            MeshFormat::Stl => qymcad_io::Format::Stl,
            MeshFormat::Obj => qymcad_io::Format::Obj,
            MeshFormat::Ply => qymcad_io::Format::Ply,
            MeshFormat::Glb => qymcad_io::Format::Gltf,
            MeshFormat::ThreeMf => qymcad_io::Format::ThreeMf,
            MeshFormat::Amf => qymcad_io::Format::Amf,
        }
    }

    /// The format a file is, by its extension; `None` for a file that is no mesh.
    pub fn of_path(path: &str) -> Option<MeshFormat> {
        match qymcad_io::Format::of_path(path)? {
            qymcad_io::Format::Stl => Some(MeshFormat::Stl),
            qymcad_io::Format::Obj => Some(MeshFormat::Obj),
            qymcad_io::Format::Ply => Some(MeshFormat::Ply),
            qymcad_io::Format::Gltf => Some(MeshFormat::Glb),
            qymcad_io::Format::ThreeMf => Some(MeshFormat::ThreeMf),
            qymcad_io::Format::Amf => Some(MeshFormat::Amf),
            _ => None,
        }
    }

    /// The file carries no unit: its numbers are millimetres only by guess.
    pub fn unitless(self) -> bool {
        matches!(self, MeshFormat::Stl | MeshFormat::Obj | MeshFormat::Ply)
    }
}

/// A piece of a mesh file on its way into the document: a piece as the file names it (see `qymcad_io::NamedMesh`) with
/// the faces found on its mesh.
pub struct MeshPiece {
    pub name: String,
    pub mesh: Mesh,
    pub faces: Vec<MeshFace>,
    pub color: Option<[u8; 3]>,
    pub place: [f64; 12],
    /// the colour of every triangle, where the file gives one
    pub tri_colors: Vec<[u8; 3]>,
    /// the groups of the file the piece stands in, from the top down
    pub within: Vec<FileGroup>,
}

/// READ A MESH FILE into its pieces, each with the faces found on its mesh. Heavy on a large mesh, and touches nothing
/// but the file: the window runs it in a thread of its own. The error is the reader's code.
pub fn read_mesh(path: &str, format: MeshFormat) -> Result<Vec<MeshPiece>, String> {
    let read = match format {
        MeshFormat::Stl => qymcad_io::import_stl_named(path),
        MeshFormat::Obj => qymcad_io::import_obj(path),
        MeshFormat::Ply => qymcad_io::import_ply_coloured(path).map(|n| vec![n]),
        MeshFormat::Glb => qymcad_io::import_gltf(path),
        MeshFormat::ThreeMf => qymcad_io::import_3mf(path),
        MeshFormat::Amf => qymcad_io::import_amf(path),
    }?;
    Ok(read
        .into_iter()
        .map(|n| {
            let faces = n.mesh.detect_faces(8.0);
            MeshPiece { name: n.name, mesh: n.mesh, faces, color: n.color, place: n.place, tri_colors: n.tri_colors, within: n.within }
        })
        .collect())
}

/// Embed the original of an imported file into the document and return its id.
pub fn embed_source(project: &mut Project, path: &str) -> Option<Id> {
    std::fs::read(path).ok().map(|bytes| project.add_source(file_name(path), bytes))
}

/// The name of the file at `path`, without its folder.
pub fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

/// The name of the file at `path`, without its folder and its extension: what an unnamed part of it is called.
pub fn stem_of(path: &str) -> String {
    let base = file_name(path);
    std::path::Path::new(&base).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(base)
}

/// WHAT CAME IN, kept beside the document while its scale may still be asked about: everything a factor is taken
/// from, at the file's own numbers.
pub struct Landed {
    /// The component the file's tree came in under; `None` when nothing was laid.
    pub root: Option<Id>,
    /// Every body that came in.
    pub bodies: Vec<Id>,
    /// The file carries no unit (STL, OBJ, PLY).
    pub unitless: bool,
    /// The sides of the box of everything that came in, as the file has it.
    pub span: [f64; 3],
    /// Every mesh that came in: its body, and the mesh and faces as read.
    pub meshes: Vec<(Id, Mesh, Vec<MeshFace>)>,
    /// Every solid that came in: its body, and the shape as read.
    pub solids: Vec<(Id, Shape)>,
    /// Every component that came in with the file's tree: its id, and where the file places it.
    pub places: Vec<(Id, [f64; 12])>,
}

/// LAY IN THE MESHES read from `path`: every piece a part, as a solid comes in, under its own name or the file's,
/// numbered, in the file's colour; the file's groups as subassemblies. Reported behaviour: a mesh came in as bodies
/// with no part, seen only at the top of the assembly.
///
/// `shown` is how a name reads to a person, so a part of the file that reads as one already here is numbered apart.
pub fn land_mesh(project: &mut Project, path: &str, format: MeshFormat, pieces: Vec<MeshPiece>, shown: &dyn Fn(&str) -> String) -> Landed {
    let span = span_of(pieces.iter().map(|p| &p.mesh));
    let read: Vec<Built> = pieces.iter().map(|p| Built { mesh: p.mesh.clone(), faces: p.faces.clone() }).collect();
    let source = embed_source(project, path).unwrap_or(0);
    let stem = stem_of(path);
    let many = pieces.len() > 1;
    let mut tops: Vec<Entry> = Vec::new();
    let mut bodies = Vec::with_capacity(pieces.len());
    for (k, MeshPiece { name: own, mesh, faces, color, place, tri_colors, within }) in pieces.into_iter().enumerate() {
        let body = project.add_mesh(mesh);
        project.set_body_faces(body, faces);
        if !own.is_empty() {
            project.set_mesh_name(project.bodies.len() - 1, own.clone());
        }
        let name = if !own.is_empty() {
            own
        } else if many {
            format!("{stem} {}", k + 1)
        } else {
            stem.clone()
        };
        put(&mut tops, &within, ImportNode { name, place, body: Some(body), solid: k as u32, color, tri_colors, mesh: true, ..Default::default() });
        bodies.push(body);
    }
    let nodes = tops.into_iter().map(|e| e.node(&stem)).collect();
    let root = project.import_tree_as_parts(nodes, source, &stem);
    let places = settle_names(project, root, shown);
    let meshes = bodies.iter().copied().zip(read).map(|(id, Built { mesh, faces })| (id, mesh, faces)).collect();
    Landed { root, bodies, unitless: format.unitless(), span, meshes, solids: Vec::new(), places }
}

/// LAY IN THE SOLIDS read from `path` as the file's tree of subassemblies and parts, each in the coordinates of its
/// own part, the live shapes into `shapes`. The import nodes are re-tessellated by the next rebuild, which also takes
/// their faces and edges from the B-rep; [`world_span`] after it gives the size as it then stands.
pub fn land_exact(project: &mut Project, shapes: &mut HashMap<Id, Shape>, path: &str, tree: qymcad_kernel::ExactTree, shown: &dyn Fn(&str) -> String) -> Landed {
    let qymcad_kernel::ExactTree { bodies: built, shapes: solids, nodes } = tree;
    let source = embed_source(project, path).unwrap_or(0);
    let stem = stem_of(path);
    let mut solids = solids.into_iter();
    let mut bodies = Vec::with_capacity(built.len());
    let mut read = Vec::with_capacity(built.len());
    for Built { mesh, faces } in built {
        let body = project.add_mesh(mesh);
        project.set_body_faces(body, faces);
        if let Some(s) = solids.next() {
            if let Some(copy) = s.transformed(&scale_matrix(1.0)) {
                read.push((body, copy)); // the shape as read, which every factor is taken from
            }
            shapes.insert(body, s);
        }
        bodies.push(body);
    }
    let root = project.import_tree_as_parts(qymcad_kernel::document_tree(&nodes, &bodies, &stem), source, &stem);
    let places = settle_names(project, root, shown);
    let span = world_span(project, &bodies);
    Landed { root, bodies, unitless: false, span, meshes: Vec::new(), solids: read, places }
}

/// A part of the file that reads as one already here ("Part 1" of the file beside "Part 1" of the document) is
/// numbered apart; and where every component that came in stands, so a factor scales the assembly about the file's
/// zero, not each part about its own.
fn settle_names(project: &mut Project, root: Option<Id>, shown: &dyn Fn(&str) -> String) -> Vec<(Id, [f64; 12])> {
    let Some(root) = root else { return Vec::new() };
    let came: Vec<Id> = std::iter::once(root).chain(project.descendants(root)).collect();
    project.name_apart(&came, shown);
    came.into_iter().map(|c| (c, project.component_transform(c))).collect()
}

/// PUT EVERYTHING THAT CAME IN AT `factor` from the file's own numbers: a mesh from its copy as read, a solid from its
/// shape as read, with the factor kept by its import node; every component that came in stands at the factor too,
/// about the file's zero. Returns whether solids changed, which then need a rebuild to be shown.
pub fn apply_scale(project: &mut Project, shapes: &mut HashMap<Id, Shape>, landed: &Landed, factor: f64) -> bool {
    let f = factor;
    for (id, mesh, faces) in &landed.meshes {
        project.set_import_scale(*id, f); // the mesh piece keeps the factor its mesh stands at
        let Some(i) = project.bodies.iter().position(|b| b.id == *id) else { continue };
        let mut m = mesh.clone();
        m.scale(0.0, 0.0, 0.0, f);
        project.bodies[i].mesh = m;
        project.set_body_faces(*id, scaled_faces(faces, f));
    }
    for (id, shape) in &landed.solids {
        if let Some(s) = shape.transformed(&scale_matrix(f)) {
            shapes.insert(*id, s);
        }
        project.set_import_scale(*id, f);
    }
    for (id, place) in &landed.places {
        let mut m = *place;
        for k in [3, 7, 11] {
            m[k] *= f;
        }
        project.set_component_transform(*id, m);
    }
    !landed.solids.is_empty()
}

/// No part a person imports is this size: under `SMALLEST_MM` or over `LARGEST_MM` on its largest side.
pub fn out_of_size(span: [f64; 3]) -> bool {
    let largest = span.iter().copied().fold(0.0, f64::max);
    !(SMALLEST_MM..=LARGEST_MM).contains(&largest)
}

/// Faces at factor `f` about the origin: a centroid moves with it, an area grows as its square.
pub fn scaled_faces(faces: &[MeshFace], f: f64) -> Vec<MeshFace> {
    faces.iter().map(|x| MeshFace { centroid: Point3::new(x.centroid.x * f, x.centroid.y * f, x.centroid.z * f), area: x.area * f * f, ..x.clone() }).collect()
}

/// The sides of the box of `bodies` as they stand in the world, each placed by its part.
pub fn world_span(project: &Project, bodies: &[Id]) -> [f64; 3] {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for &id in bodies {
        let Some(b) = project.mesh_index(id).and_then(|i| project.bodies[i].mesh.bounds()) else { continue };
        let w = project.body_world_transform(id);
        for corner in 0..8 {
            let p = [if corner & 1 == 0 { b.min.x } else { b.max.x }, if corner & 2 == 0 { b.min.y } else { b.max.y }, if corner & 4 == 0 { b.min.z } else { b.max.z }];
            for r in 0..3 {
                let v = w[r * 4] * p[0] + w[r * 4 + 1] * p[1] + w[r * 4 + 2] * p[2] + w[r * 4 + 3];
                lo[r] = lo[r].min(v);
                hi[r] = hi[r].max(v);
            }
        }
    }
    std::array::from_fn(|k| (hi[k] - lo[k]).max(0.0))
}

/// The sides of the box of every mesh.
pub fn span_of<'a>(meshes: impl Iterator<Item = &'a Mesh>) -> [f64; 3] {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for b in meshes.filter_map(Mesh::bounds) {
        for (k, (a, z)) in [(b.min.x, b.max.x), (b.min.y, b.max.y), (b.min.z, b.max.z)].into_iter().enumerate() {
            lo[k] = lo[k].min(a);
            hi[k] = hi[k].max(z);
        }
    }
    std::array::from_fn(|k| (hi[k] - lo[k]).max(0.0))
}

/// A piece of a mesh file, or a group of the file with what stands in it, on the way to the document's tree.
enum Entry {
    Piece(ImportNode),
    Group { id: usize, name: String, place: [f64; 12], kids: Vec<Entry> },
}

/// `leaf` into `level`, under the groups `chain` names from the top down, each made where it is first met.
fn put(level: &mut Vec<Entry>, chain: &[FileGroup], leaf: ImportNode) {
    let Some((FileGroup { index: id, name, place }, rest)) = chain.split_first() else {
        level.push(Entry::Piece(leaf));
        return;
    };
    let at = match level.iter().position(|e| matches!(e, Entry::Group { id: g, .. } if g == id)) {
        Some(at) => at,
        None => {
            level.push(Entry::Group { id: *id, name: name.clone(), place: *place, kids: Vec::new() });
            level.len() - 1
        }
    };
    if let Entry::Group { kids, .. } = &mut level[at] {
        put(kids, rest, leaf);
    }
}

impl Entry {
    /// The document's node: a group the file leaves unnamed takes the file's name, as an unnamed piece does.
    fn node(self, stem: &str) -> ImportNode {
        match self {
            Entry::Piece(n) => n,
            Entry::Group { name, place, kids, .. } => {
                ImportNode { name: if name.is_empty() { stem.to_string() } else { name }, place, children: kids.into_iter().map(|k| k.node(stem)).collect(), ..Default::default() }
            }
        }
    }
}
