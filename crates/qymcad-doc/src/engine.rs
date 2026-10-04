//! ONE DOCUMENT, DRIVEN WITHOUT A WINDOW: opened, changed one action at a time, taken back and put again, written
//! out. Every step is the step the window takes - the same rebuild, the same undo boundary, the same live bodies -
//! only with no camera, no selection and no status line around it.
//!
//! THE DOOR IS ONE: the document is read freely and changed only through [`DocEngine::edit`]. A change made past it
//! would leave no undo step and no rebuild, and the next reading would measure a document nobody built.
use crate::{brep, export, history, import, regen};
use qymcad_core::feature::{FeatureKind, RegenReport};
use qymcad_core::model::{Id, Project};
use qymcad_kernel::Shape;
use std::collections::HashMap;

/// How many steps undo keeps: the window's own default.
pub const UNDO_DEPTH: usize = 40;

/// Why the document was not opened, written or changed. A code, not a phrase: the caller knows the language.
#[derive(Debug, Clone, PartialEq)]
pub enum DocError {
    /// The file could not be read or written: the code of the file layer, e.g. `io-file-read#<path>: <reason>`.
    File(String),
    /// The action laid nodes on what the document no longer holds; those nodes, each of which would only stand red.
    /// The action is taken back whole.
    Gone(Vec<Id>),
    /// The action itself refused, with its own code. The action is taken back whole.
    Refused(String),
}

/// What a file brought in.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    /// The component the file's tree came in under.
    pub root: Option<Id>,
    /// Every body that came in.
    pub bodies: Vec<Id>,
    /// The file carries no unit: its numbers are millimetres only by guess.
    pub unitless: bool,
    /// The sides of the box of everything that came in, as the file has it, before the factor.
    pub span: [f64; 3],
}

/// What a file was written with.
#[derive(Debug, Clone, PartialEq)]
pub struct Exported {
    pub written: export::Written,
    /// The bodies sorted by what could go into the file: into an exact file only `brep` went; into a mesh file the rest
    /// went as their stored meshes.
    pub plan: export::Plan,
}

/// THE DOCUMENT AND EVERYTHING LIVE BESIDE IT.
pub struct DocEngine {
    project: Project,
    /// The live B-rep of each body, by body id.
    shapes: HashMap<Id, Shape>,
    /// The live bodies and source bytes of imports taken back by undo, kept for redo.
    shelved: HashMap<Id, Shape>,
    shelved_sources: HashMap<Id, Vec<u8>>,
    /// The parameter values the last rebuild was made with.
    params_seen: HashMap<String, f64>,
    history: history::History,
    /// What the last rebuild answered: a red node stays in the document, as in the window, and is told here.
    report: RegenReport,
}

impl DocEngine {
    /// A document taken as it is: whatever it marks dirty is built now.
    pub fn new(project: Project) -> Self {
        let mut d = DocEngine {
            project,
            shapes: HashMap::new(),
            shelved: HashMap::new(),
            shelved_sources: HashMap::new(),
            params_seen: HashMap::new(),
            history: history::History::new(UNDO_DEPTH),
            report: RegenReport::default(),
        };
        d.params_seen = d.project.param_map();
        d.rebuild();
        d
    }

    /// A new empty document: the root assembly and one part, as the window starts.
    pub fn blank() -> Self {
        let mut p = Project::default();
        p.new_document();
        Self::new(p)
    }

    /// OPEN A `.qcad` BUNDLE as the window opens one.
    ///
    /// The geometry comes from the file rather than from a rebuild: the live bodies written into it go into the cache,
    /// the faces back where references resolve them, and only what the file carries no geometry for is rebuilt. The
    /// parameters count as seen at their stored values - the geometry in the file was built from exactly those. The
    /// imports get their live bodies from their embedded sources, at the factor each node keeps. A raw mesh with no
    /// B-rep gets its faces found on the mesh.
    pub fn open(path: &str) -> Result<Self, DocError> {
        let qymcad_io::LoadedProject { mut project, breps } = qymcad_io::load_project_with_brep(path).map_err(DocError::File)?;
        project.ensure_document();
        let from_file = brep::shapes_from_blobs(breps);
        let mut shapes = HashMap::new();
        let adopted = brep::adopt_loaded(&mut project, &mut shapes, from_file);
        if adopted.imports {
            shapes.extend(brep::import_shapes(&project));
        }
        let mut d = DocEngine {
            params_seen: project.param_map(),
            project,
            shapes,
            shelved: HashMap::new(),
            shelved_sources: HashMap::new(),
            history: history::History::new(UNDO_DEPTH),
            report: RegenReport::default(),
        };
        d.rebuild();
        let _ = brep::detect_missing_faces(&mut d.project, &d.shapes);
        Ok(d)
    }

    /// WRITE THE DOCUMENT TO `path`, with its live bodies, so that opening it needs no rebuild.
    ///
    /// The faces kept for references are derived from the faces of the bodies and are not written twice. The write is
    /// guarded: an empty document over a file with content is refused, not a loss. `now` is the caller's clock, an
    /// ISO-8601 stamp: the document takes it as the moment it was started on its first save, and never again.
    pub fn save(&mut self, path: &str, now: &str) -> Result<(), DocError> {
        if self.project.meta.created.is_empty() {
            self.project.meta.created = now.to_string();
        }
        let mut out = self.project.clone();
        out.regen_faces.clear();
        out.regen_edges.clear();
        let breps: Vec<(Id, Vec<u8>)> = {
            let _gate = qymcad_kernel::kernel_gate();
            self.shapes.iter().filter_map(|(id, s)| s.to_brep_bytes().map(|b| (*id, b))).collect()
        };
        qymcad_io::save_project_guarded_with_brep(&out, path, &breps).map_err(DocError::File)
    }

    /// ONE ACTION UNDER `name`: `f` changes the document, the rebuild follows, and the whole is one undo step.
    ///
    /// The live bodies are brought up first, where the action may need them. An import the action lays gets its live
    /// body from its source. A refusal from `f`, or a node laid on what the document no longer holds, takes the
    /// action back whole and leaves no step. A node that does not build stays red in the document, as in the window,
    /// and is told in [`DocEngine::report`].
    pub fn edit<R>(&mut self, name: &str, f: impl FnOnce(&mut Project) -> Result<R, String>) -> Result<R, DocError> {
        self.ensure_brep();
        let before: Vec<Id> = self.project.timeline.iter().map(|n| n.id).collect();
        self.history.begin(name, &self.project);
        let r = match f(&mut self.project) {
            Ok(r) => r,
            Err(code) => {
                self.abort();
                return Err(DocError::Refused(code));
            }
        };
        let fresh: Vec<Id> = self.project.timeline.iter().map(|n| n.id).filter(|id| !before.contains(id)).collect();
        let gone: Vec<Id> = fresh.into_iter().filter(|&node| !self.project.gone_inputs(node).is_empty()).collect();
        if !gone.is_empty() {
            self.abort();
            return Err(DocError::Gone(gone));
        }
        self.raise_imports();
        self.rebuild();
        self.history.commit();
        Ok(r)
    }

    /// BRING A FILE IN as one action: a mesh (STL, OBJ, PLY, glTF, 3MF, AMF) or a solid (STEP, IGES), laid in as the
    /// file's tree of parts with its original embedded, taken at `factor` from the file's own numbers. The file is
    /// read before the action opens, so a file that cannot be read leaves no step.
    ///
    /// `shown` is how a name reads to a person, so a part of the file that reads as one already here is numbered
    /// apart. The answer says what came in and how big the file has it, for the caller to judge the factor by
    /// ([`import::out_of_size`]).
    pub fn import(&mut self, path: &str, factor: f64, shown: &dyn Fn(&str) -> String) -> Result<Imported, DocError> {
        enum Read {
            Mesh(import::MeshFormat, Vec<import::MeshPiece>),
            Exact(qymcad_kernel::ExactTree),
        }
        let read = if let Some(format) = import::MeshFormat::of_path(path) {
            Read::Mesh(format, import::read_mesh(path, format).map_err(DocError::File)?)
        } else {
            let format = match qymcad_io::Format::of_path(path) {
                Some(qymcad_io::Format::Step) => qymcad_kernel::ExactFormat::Step,
                Some(qymcad_io::Format::Iges) => qymcad_kernel::ExactFormat::Iges,
                _ => return Err(DocError::File(format!("import-unknown#{}", import::file_name(path)))),
            };
            let tree = qymcad_kernel::read_exact_tree(format, path, 0.5).map_err(DocError::File)?;
            if tree.bodies.is_empty() {
                let named = qymcad_io::Format::of_path(path).map(|f| f.name()).unwrap_or_default();
                return Err(DocError::File(format!("io-exact-no-solids#{named}")));
            }
            Read::Exact(tree)
        };
        self.ensure_brep();
        self.history.begin("name-import", &self.project); // a catalogue key: the step is named in the person's language
        let landed = match read {
            Read::Mesh(format, pieces) => import::land_mesh(&mut self.project, path, format, pieces, shown),
            Read::Exact(tree) => import::land_exact(&mut self.project, &mut self.shapes, path, tree, shown),
        };
        if (factor - 1.0).abs() > 1e-12 && factor > 0.0 {
            let _ = import::apply_scale(&mut self.project, &mut self.shapes, landed.as_read(), factor);
        }
        self.rebuild();
        self.history.commit();
        Ok(Imported { root: landed.root, bodies: landed.bodies, unitless: landed.unitless, span: landed.span })
    }

    /// WRITE `target` AS A MESH FILE at `deflection` mm: every visible body, a live B-rep tessellated to that detail,
    /// a body with none as its stored mesh. glTF and 3MF go out as the tree of parts under the names `shown` gives,
    /// placed, in their colours; the rest flat. The answer tells what went as a mesh only.
    pub fn export_mesh(&mut self, path: &str, format: import::MeshFormat, target: export::ExportTarget, deflection: f64, shown: &dyn Fn(&str) -> String) -> Result<Exported, DocError> {
        self.ensure_brep();
        let plan = export::plan(&self.project, target, |b| self.shapes.contains_key(&b));
        let bodies = plan.mesh_bodies();
        if bodies.is_empty() {
            return Err(DocError::File(format!("io-mesh-no-bodies#{}", format.entry().name())));
        }
        let out = export::MeshOut { format, deflection, tree: export::mesh_tree(&self.project, format, target, &bodies, shown) };
        let solids = export::placed_solids(&self.project, &self.shapes, &bodies);
        let stored = bodies.iter().filter(|b| !self.shapes.contains_key(b)).filter_map(|&b| export::stored_mesh(&self.project, b)).collect();
        let written = export::write_mesh(&out, &solids, stored, path).map_err(DocError::File)?;
        Ok(Exported { written, plan })
    }

    /// WRITE `target` AS AN EXACT FILE: every visible body with a live B-rep; a STEP as the document's tree under the
    /// names `shown` gives, an IGES flat. A body with none - a mesh, a failed rebuild - does not reach the file, and the
    /// answer tells which.
    pub fn export_exact(&mut self, path: &str, format: qymcad_kernel::ExactFormat, target: export::ExportTarget, shown: &dyn Fn(&str) -> String) -> Result<Exported, DocError> {
        self.ensure_brep();
        let plan = export::plan(&self.project, target, |b| self.shapes.contains_key(&b));
        if plan.brep.is_empty() {
            let named = match format {
                qymcad_kernel::ExactFormat::Step => qymcad_io::Format::Step,
                qymcad_kernel::ExactFormat::Iges => qymcad_io::Format::Iges,
            }
            .name();
            let code = if plan.mesh_only.len() + plan.stale.len() > 0 { "io-exact-no-brep" } else { "io-exact-no-bodies" };
            return Err(DocError::File(format!("{code}#{named}")));
        }
        let tree = export::exact_tree(&self.project, format, target, &plan.brep, shown);
        let solids = export::placed_solids(&self.project, &self.shapes, &plan.brep);
        let written = export::write_exact(format, &tree, &solids, path).map_err(DocError::File)?;
        Ok(Exported { written, plan })
    }

    /// One step back; the name of the step taken back, or nothing when there is none.
    pub fn undo(&mut self) -> Option<String> {
        let (name, snap) = self.history.undo(&self.project)?;
        self.put(snap);
        Some(name)
    }

    /// One step forward; the name of the step put again, or nothing when there is none.
    pub fn redo(&mut self) -> Option<String> {
        let (name, snap) = self.history.redo(&self.project)?;
        self.put(snap);
        Some(name)
    }

    /// BRING THE LIVE BODIES UP: the nodes whose bodies are shown from the file's mesh alone are rebuilt, nothing else.
    /// Derived work - the document does not change, and no step is made.
    pub fn ensure_brep(&mut self) {
        if brep::mark_missing_brep(&mut self.project, &self.shapes) > 0 {
            self.rebuild();
        }
    }

    /// Rebuild what is marked, and what reads a parameter that changed since the last rebuild.
    pub fn rebuild(&mut self) -> &RegenReport {
        self.report = regen::run(&mut self.project, &mut self.shapes, &mut self.params_seen, None).report;
        &self.report
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The live B-rep of `body`, when it is up.
    pub fn shape(&self, body: Id) -> Option<&Shape> {
        self.shapes.get(&body)
    }

    /// What the last rebuild answered.
    pub fn report(&self) -> &RegenReport {
        &self.report
    }

    pub fn history(&self) -> &history::History {
        &self.history
    }

    /// Take the open action back: the state before it is restored and rebuilt, no step is left.
    fn abort(&mut self) {
        if let Some(snap) = self.history.abort() {
            self.put(snap);
        }
    }

    /// Restore a state from the history and rebuild what it changed.
    fn put(&mut self, snap: Project) {
        let shelf = history::Shelf { shapes: &mut self.shelved, sources: &mut self.shelved_sources };
        let _ = history::restore_snapshot(&mut self.project, &mut self.shapes, shelf, snap);
        self.rebuild();
    }

    /// The imports with no live body yet get one from their sources; an import has no recipe to build one from.
    fn raise_imports(&mut self) {
        let waiting = self.project.timeline.iter().any(|n| matches!(n.kind, FeatureKind::Import { body, .. } if !self.shapes.contains_key(&body)));
        if waiting {
            for (body, s) in brep::import_shapes(&self.project) {
                self.shapes.entry(body).or_insert(s);
            }
        }
    }
}
