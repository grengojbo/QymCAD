//! ONE DOCUMENT, DRIVEN WITHOUT A WINDOW: opened, changed one action at a time, taken back and put again, written
//! out. Every step is the step the window takes - the same rebuild, the same undo boundary, the same live bodies -
//! only with no camera, no selection and no status line around it.
//!
//! THE DOOR IS ONE: the document is read freely and changed only through [`DocEngine::edit`]. A change made past it
//! would leave no undo step and no rebuild, and the next reading would measure a document nobody built.
use crate::{brep, history, regen};
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
