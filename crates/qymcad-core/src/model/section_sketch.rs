//! A SKETCH DRAWN FROM LOOPS FOUND ELSEWHERE: a section of a mesh fitted with lines, arcs, circles and splines
//! (`qymcad_meshfit::section`) laid into a new sketch as curves of its own and defined to the end. A mesh has no sketch
//! behind it and no exact edge to project; its section is the drawing a person would trace by hand.
use super::{Anchor, FaceSketch, Project, Seen};
use crate::feature::{Ends, Purpose, SketchPlane};
use crate::geom::{Fitted, Piece};

/// WHAT A SKETCH IS DRAWN FROM: the plane it lies on, the loops in the coordinates of that plane, and its name.
pub struct FromLoops<'a> {
    pub plane: SketchPlane,
    pub loops: &'a [Fitted],
    pub name: String,
}

impl Project {
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
