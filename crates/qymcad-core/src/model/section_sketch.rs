//! A SKETCH DRAWN FROM LOOPS FOUND ELSEWHERE: a section of a mesh fitted with lines, arcs, circles and splines
//! (`qymcad_meshfit::section`) laid into a new sketch as curves of its own and defined to the end. A mesh has no sketch
//! behind it and no exact edge to project; its section is the drawing a person would trace by hand.
use super::{Anchor, FaceSketch, Id, PlaneDef, Project, Seen, WorkPlane};
use crate::feature::{BasePlane, Ends, PlaneFrame, Purpose, SketchPlane};
use crate::geom::{Fitted, Mesh, Piece};

/// WHAT FINDS THE LOOPS OF A SECTION: the mesh of a body and the frame of the plane cutting it, to the loops fitted in
/// that frame (`qymcad_meshfit::section`). The core cuts nothing itself: what a mesh is made of is another module's task.
pub type Fitter = dyn Fn(&Mesh, &PlaneFrame) -> Vec<Fitted>;

/// WHERE A SECTION IS TAKEN: a base plane, moved by `offset` the way a datum plane off it moves
/// (`BasePlane::offset_direction`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionAt {
    pub base: BasePlane,
    pub offset: f64,
}

/// An offset this small is none: the sketch lies on the base plane itself, and no datum plane is laid.
const NO_OFFSET: f64 = 1e-9;

impl SectionAt {
    /// The frame the section is fitted in and its sketch is drawn in: the base plane's own, or that of the datum plane
    /// laid off it - one frame, so what the tool shows before Enter is what the sketch holds after it.
    pub fn frame(&self) -> PlaneFrame {
        let f = self.base.frame();
        if self.offset.abs() < NO_OFFSET {
            return f;
        }
        let d = self.base.offset_direction();
        PlaneFrame::world_aligned([f.origin[0] + d[0] * self.offset, f.origin[1] + d[1] * self.offset, f.origin[2] + d[2] * self.offset], f.normal(), 0.0)
    }
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

impl Project {
    /// A SKETCH OF A SECTION OF A BODY'S MESH: the mesh cut and fitted (`OfSection::fit`) in the frame of `at`, a datum
    /// plane laid off the base plane where the offset is not none, and the loops drawn into a sketch on it, defined to
    /// the end (`add_sketch_of_loops`). Answers `None` and lays nothing when the plane misses the body.
    pub fn add_sketch_of_section(&mut self, of: OfSection) -> Option<FaceSketch> {
        let OfSection { body, at, fit, name } = of;
        let mesh = self.bodies.iter().find(|b| b.id == body).map(|b| b.mesh.clone())?;
        let frame = at.frame();
        let loops = fit(&mesh, &frame);
        if loops.iter().all(|l| l.pieces.is_empty()) {
            return None;
        }
        let plane = if at.offset.abs() < NO_OFFSET {
            SketchPlane::World(at.base)
        } else {
            let id = self.add_plane(WorkPlane { origin: frame.origin, normal: frame.normal(), def: PlaneDef::OffsetBase { base: at.base, dist: at.offset }, ..Default::default() });
            SketchPlane::Datum(id)
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
        Some(FaceSketch { si, sketch, defined })
    }
}
