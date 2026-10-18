//! A SKETCH DRAWN FROM LOOPS FOUND ELSEWHERE: a section of a mesh fitted with lines, arcs, circles and splines
//! (`qymcad_meshfit::section`) laid into a new sketch as curves of its own and defined to the end. A mesh has no sketch
//! behind it and no exact edge to project; its section is the drawing a person would trace by hand.
use super::{Anchor, FaceSketch, Id, PlaneDef, Project, Seen, WorkPlane};
use crate::feature::{Ends, PlaneFrame, Purpose, SketchPlane};
use crate::geom::{Fitted, Mesh, Piece};

/// WHAT FINDS THE LOOPS OF A SECTION: the mesh of a body and the frame of the plane cutting it, to the loops fitted in
/// that frame (`qymcad_meshfit::section`). The core cuts nothing itself: what a mesh is made of is another module's task.
pub type Fitter = dyn Fn(&Mesh, &PlaneFrame) -> Vec<Fitted>;

/// WHERE A SECTION IS TAKEN: a base plane, a datum plane or a flat face, moved by `offset` the way a datum plane off it
/// moves - off a base plane along `BasePlane::offset_direction`, off a datum plane or a face along its normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionAt {
    pub on: SketchPlane,
    pub offset: f64,
}

/// An offset this small is none: the sketch lies on the plane itself, and no datum plane is laid.
const NO_OFFSET: f64 = 1e-9;

/// A plane as a datum plane off it stands: a point of it, its unit normal, the way an offset moves it, and the turn of
/// its X axis.
struct Source {
    origin: [f64; 3],
    normal: [f64; 3],
    along: [f64; 3],
    rot_deg: f64,
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// WHAT FITS ONE CLOSED LOOP: its corners in order, to its pieces (`qymcad_meshfit::section::fit_loop`).
pub type LoopFitter = dyn Fn(&[crate::geom::Point2]) -> Fitted;

/// WHAT A SKETCH OF A FACE OF A MESH IS TAKEN OF: the body, its flat face, what fits the loops of its outline, the name.
pub struct OfFace<'a> {
    pub body: Id,
    pub face: crate::feature::FaceKey,
    pub fit: &'a LoopFitter,
    pub name: String,
}

/// WHAT A SKETCH OF A SECTION IS TAKEN OF: the body, where it is cut, what fits the loops, and the sketch's name.
pub struct OfSection<'a> {
    pub body: Id,
    pub at: SectionAt,
    pub fit: &'a Fitter,
    pub name: String,
}

/// WHAT A SKETCH IS DRAWN FROM: the plane it lies on, the loops in the coordinates of that plane, and its name.
pub struct FromLoops<'a> {
    pub plane: SketchPlane,
    pub loops: &'a [Fitted],
    pub name: String,
}

/// WHERE A BODY BEGINS AND ENDS along the way an offset off a base plane moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub from: f64,
    pub to: f64,
}

impl Project {
    /// WHETHER A BODY CAME FROM A MESH: the node that made it, or one it was made from, brought a mesh in or sewed one into
    /// a polyhedron. A plate printed and brought back, its hole since made exact, is still drawn by its section and by
    /// its faces' fitted outlines; a body of exact surfaces from the start has exact edges to take instead.
    pub fn came_from_a_mesh(&self, mut body: Id) -> bool {
        use crate::feature::FeatureKind;
        for _ in 0..self.timeline.len() + 1 {
            let Some(node) = self.timeline.iter().find(|n| n.kind.body() == Some(body)) else { return false };
            match node.kind {
                FeatureKind::MeshPiece { .. } | FeatureKind::MeshSolid { .. } => return true,
                _ => match node.kind.consumed_body() {
                    Some(src) if src != body => body = src,
                    _ => return false,
                },
            }
        }
        false
    }

    /// The plane `on` as a datum plane off it would stand (`resolve_plane_into`). None for a datum plane or a body that
    /// is not there.
    fn source_of(&self, on: &SketchPlane) -> Option<Source> {
        match *on {
            SketchPlane::World(b) => {
                let f = b.frame();
                Some(Source { origin: f.origin, normal: f.normal(), along: b.offset_direction(), rot_deg: 0.0 })
            }
            SketchPlane::Datum(id) => self.planes.iter().find(|p| p.id == id).map(|p| {
                let n = unit(p.normal);
                Source { origin: p.origin, normal: n, along: n, rot_deg: p.rot_deg }
            }),
            SketchPlane::Face(body, key) => {
                self.bodies.iter().find(|b| b.id == body)?;
                let (c, n) = self.resolve_face(body, &key);
                let n = unit(n);
                Some(Source { origin: c, normal: n, along: n, rot_deg: 0.0 })
            }
        }
    }

    /// THE FRAME A SECTION IS FITTED IN AND ITS SKETCH IS DRAWN IN: that of the plane it is taken on, or of the datum
    /// plane laid off it - one frame, so what the tool shows before Enter is what the sketch holds after it.
    pub fn section_frame(&self, at: &SectionAt) -> Option<PlaneFrame> {
        if at.offset.abs() < NO_OFFSET {
            if let SketchPlane::World(b) = at.on {
                return Some(b.frame());
            }
        }
        let s = self.source_of(&at.on)?;
        let o = [s.origin[0] + s.along[0] * at.offset, s.origin[1] + s.along[1] * at.offset, s.origin[2] + s.along[2] * at.offset];
        Some(PlaneFrame::world_aligned(o, s.normal, s.rot_deg))
    }

    /// WHERE `body` BEGINS AND ENDS along the way an offset off `on` moves (`SectionAt`), by the corners of its mesh.
    pub fn extent_along(&self, body: Id, on: &SketchPlane) -> Option<Extent> {
        let mesh = &self.bodies.iter().find(|b| b.id == body)?.mesh;
        let s = self.source_of(on)?;
        let along = |p: &crate::geom::Point3| (p.x - s.origin[0]) * s.along[0] + (p.y - s.origin[1]) * s.along[1] + (p.z - s.origin[2]) * s.along[2];
        let (lo, hi) = mesh.verts.iter().map(along).fold((f64::MAX, f64::MIN), |(lo, hi), d| (lo.min(d), hi.max(d)));
        (lo <= hi).then_some(Extent { from: lo, to: hi })
    }

    /// A SKETCH OF A SECTION OF A BODY'S MESH: the mesh cut and fitted (`OfSection::fit`) in the frame of `at`, and the
    /// loops drawn into a sketch, defined to the end (`add_sketch_of_loops`). The sketch lies on the plane clicked when
    /// the offset is none and it is a base or a datum plane; otherwise on a datum plane laid off it - off the base plane,
    /// the datum plane or the face - which moves with what it was laid off. Answers `None` and lays nothing when the
    /// plane misses the body.
    pub fn add_sketch_of_section(&mut self, of: OfSection) -> Option<FaceSketch> {
        let OfSection { body, at, fit, name } = of;
        let mesh = self.bodies.iter().find(|b| b.id == body).map(|b| b.mesh.clone())?;
        let frame = self.section_frame(&at)?;
        let loops = fit(&mesh, &frame);
        if loops.iter().all(|l| l.pieces.is_empty()) {
            return None;
        }
        let none = at.offset.abs() < NO_OFFSET;
        let def = match at.on {
            SketchPlane::World(_) | SketchPlane::Datum(_) if none => None,
            SketchPlane::World(base) => Some(PlaneDef::OffsetBase { base, dist: at.offset }),
            SketchPlane::Datum(plane) => Some(PlaneDef::OffsetPlane { plane, dist: at.offset }),
            SketchPlane::Face(body, face) => Some(PlaneDef::OffsetFace { body, face, dist: at.offset }),
        };
        let plane = match def {
            None => at.on,
            Some(def) => {
                let rot_deg = self.source_of(&at.on).map_or(0.0, |s| s.rot_deg);
                SketchPlane::Datum(self.add_plane(WorkPlane { origin: frame.origin, normal: frame.normal(), rot_deg, def, ..Default::default() }))
            }
        };
        self.add_sketch_of_loops(FromLoops { plane, loops: &loops, name })
    }

    /// DRAW LOOPS INTO A NEW SKETCH: every piece a curve of its own - a line, an arc, a circle, a spline through its
    /// points - the pieces of a loop joined end to end, and the drawing defined to the end from the origin of the
    /// sketch, its relations taken as a mesh shows them (`Seen::MESH`). The points of a spline are left as they stand:
    /// no size holds them. Answers `None` and lays nothing when the loops hold no piece.
    pub fn add_sketch_of_loops(&mut self, from: FromLoops) -> Option<FaceSketch> {
        let FromLoops { plane, loops, name } = from;
        if loops.iter().all(|l| l.pieces.is_empty()) {
            return None;
        }
        let si = self.new_sketch(name.clone());
        self.sketches[si].plane = plane;
        Some(self.draw_loops(si, loops, name))
    }

    /// A SKETCH OF A FLAT FACE OF A MESH: a sketch on the face, the outline of the face read in its frame, stitched into
    /// loops, each fitted (`fit`) and drawn as `add_sketch_of_loops` draws them. The outline of a mesh face is chords:
    /// copied as it is, a hole of 10 was 24 lines. Answers `None` and lays nothing when the face gives no closed loop.
    pub fn add_sketch_of_face_loops(&mut self, of: OfFace) -> Option<FaceSketch> {
        let OfFace { body, face, fit, name } = of;
        let b = self.bodies.iter().find(|b| b.id == body)?;
        let near = |f: &&crate::geom::MeshFace| {
            let c = [f.centroid.x - face.centroid[0], f.centroid.y - face.centroid[1], f.centroid.z - face.centroid[2]];
            (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt() - (f.normal[0] * face.normal[0] + f.normal[1] * face.normal[1] + f.normal[2] * face.normal[2])
        };
        // a face of a polyhedron by its name; a face of a raw mesh has none (id 0) and is the one standing where the key
        // says, turned as it says
        let mf = match face.id {
            0 => b.faces.iter().min_by(|x, y| near(x).total_cmp(&near(y)))?,
            id => b.faces.iter().find(|f| f.id == id)?,
        };
        let outline = b.mesh.face_outline(mf);
        let si = self.new_sketch(name.clone());
        self.sketches[si].plane = SketchPlane::Face(body, face);
        let Some(frame) = self.sketch_frame(si) else {
            self.sketches.pop();
            return None;
        };
        let segs = outline.iter().map(|s| (frame.project(s.from), frame.project(s.to))).collect();
        let loops: Vec<Fitted> = crate::geom::stitch_segments(segs, 1e-6).into_iter().filter(|c| c.closed && c.points.len() >= 3).map(|c| fit(&c.points)).collect();
        if loops.iter().all(|l| l.pieces.is_empty()) {
            self.sketches.pop();
            return None;
        }
        Some(self.draw_loops(si, &loops, name))
    }

    /// The pieces of `loops` drawn into sketch `si` as curves of its own, joined end to end, the sketch laid in the
    /// timeline and defined to the end from its origin, its relations taken as a mesh shows them.
    fn draw_loops(&mut self, si: usize, loops: &[Fitted], name: String) -> FaceSketch {
        let mut ents = Vec::new();
        for piece in loops.iter().flat_map(|l| &l.pieces) {
            match piece {
                Piece::Line { from, to } => ents.push(self.add_line_entity(si, from.x, from.y, to.x, to.y, Purpose::Real)),
                Piece::Arc { center, from, to, winding, .. } => {
                    self.add_arc_entity(si, *center, *from, *to, *winding, Purpose::Real);
                    ents.extend(self.sketches[si].entities.last().map(|e| e.id));
                }
                Piece::Circle { center, radius } => ents.push(self.add_circle_entity(si, center.x, center.y, *radius, Purpose::Real)),
                Piece::Spline(points) => {
                    let closed = points.len() > 2 && points.first().zip(points.last()).is_some_and(|(a, b)| a.dist(*b) < 1e-9);
                    let mut pts = points.clone();
                    if closed {
                        pts.pop();
                    }
                    self.add_spline(si, pts, if closed { Ends::Closed } else { Ends::Open }, Purpose::Real);
                }
            }
        }
        // the end of a spline and the end of the line it runs on into are one corner under two names
        self.merge_close_points(si, 1e-6);
        let sketch = self.sketches[si].id;
        self.add_sketch_node(sketch, name);
        let defined = self.fully_define_seen(si, &ents, &Anchor::Origin, Seen::MESH);
        self.regen_sketch(si);
        FaceSketch { si, sketch, defined }
    }
}
