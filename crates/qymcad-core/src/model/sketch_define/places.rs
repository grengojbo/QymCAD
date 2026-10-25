//! THE PLACE OF A DRAWING MEASURED BESIDE IT: a distance from an axis or from the edge of the face a drawing stands on
//! is a perpendicular from a point of the drawing to the line, and laid as it falls it runs along a side of the drawing
//! or across the outline next to it. It is shifted along the line, as on paper, out past the drawing - a step at a
//! time, either side - to the first place where it keeps clear of every other curve and every dimension line.
use super::*;

/// How many steps out a place line is looked for on each side before it is left at the first.
const TRIES: usize = 40;

/// A STRETCH OF A STRAIGHT LINE IN THE SKETCH: a dimension line, or a side it keeps clear of.
#[derive(Clone, Copy, Debug)]
pub(super) struct Stretch {
    pub(super) from: Point2,
    pub(super) to: Point2,
}

impl Stretch {
    /// Which side of the stretch `q` lies on, scaled by its length: the sign of the turn from it to `q`.
    fn side_of(self, q: Point2) -> f64 {
        (self.to.x - self.from.x) * (q.y - self.from.y) - (self.to.y - self.from.y) * (q.x - self.from.x)
    }

    /// How far `q` stands from the stretch.
    fn reach(self, q: Point2) -> f64 {
        let (dx, dy) = (self.to.x - self.from.x, self.to.y - self.from.y);
        let l2 = (dx * dx + dy * dy).max(1e-18);
        let t = (((q.x - self.from.x) * dx + (q.y - self.from.y) * dy) / l2).clamp(0.0, 1.0);
        (q.x - self.from.x - t * dx).hypot(q.y - self.from.y - t * dy)
    }

    /// How far the stretch keeps from `o`: nothing when they cross.
    fn gap(self, o: Stretch) -> f64 {
        if o.side_of(self.from) * o.side_of(self.to) < 0.0 && self.side_of(o.from) * self.side_of(o.to) < 0.0 {
            return 0.0;
        }
        [self.reach(o.from), self.reach(o.to), o.reach(self.from), o.reach(self.to)].into_iter().fold(f64::INFINITY, f64::min)
    }
}

/// Where a place line is shifted to along its line, and where it then runs.
struct Stood {
    off: f64,
    line: Stretch,
}

/// A circle or an arc a place line keeps clear of, taken whole.
struct Ring {
    at: Point2,
    r: f64,
}

/// WHAT A PLACE LINE KEEPS CLEAR OF: the sides and the circles of the sketch that are not of the drawing, and the
/// dimension lines standing - by a share of a step, room for the text between two lines.
pub(super) struct Around {
    sides: Vec<Stretch>,
    rings: Vec<Ring>,
    pub(super) lines: Vec<Stretch>,
    room: f64,
}

impl Around {
    fn clear(&self, l: Stretch) -> bool {
        self.sides.iter().all(|s| s.gap(l) > SAME) && self.rings.iter().all(|r| l.reach(r.at) > r.r + SAME) && self.lines.iter().all(|d| d.gap(l) > self.room)
    }
}

/// The box a drawing fills, its circles and arcs taken whole.
pub(super) struct Bounds {
    pub(super) lo: Point2,
    pub(super) hi: Point2,
}

impl Bounds {
    pub(super) fn of(c: &Curves) -> Option<Bounds> {
        let mut pts: Vec<Point2> = c.sides.iter().flat_map(|s| [s.from, s.to]).collect();
        pts.extend(c.rounds.iter().flat_map(|r| [Point2::new(r.at.x - r.r, r.at.y - r.r), Point2::new(r.at.x + r.r, r.at.y + r.r)]));
        let first = *pts.first()?;
        let lo = pts.iter().fold(first, |m, q| Point2::new(m.x.min(q.x), m.y.min(q.y)));
        let hi = pts.iter().fold(first, |m, q| Point2::new(m.x.max(q.x), m.y.max(q.y)));
        Some(Bounds { lo, hi })
    }

    /// A step between two dimension lines: 15 % of the larger side, never under a millimetre.
    pub(super) fn step(&self) -> f64 {
        ((self.hi.x - self.lo.x).max(self.hi.y - self.lo.y) * 0.15).max(1.0)
    }
}

/// The line a dimension is drawn on, in the sketch, as `sketch_marks` draws it: a width or a height along its axis at
/// the middle of its ends shifted by `off` - down the sketch for a width - and a distance from a line as the
/// perpendicular from the point to its foot, shifted along the line by `off`.
pub(super) fn dim_line(s: &crate::model::Sketch, c: &Constraint) -> Option<Stretch> {
    let at = |id: Id| s.points.iter().find(|q| q.id == id).map(|q| Point2::new(q.x, q.y));
    match *c {
        Constraint::Distance { a, b, off, axis: 1, .. } => {
            let (pa, pb) = (at(a)?, at(b)?);
            let y = (pa.y + pb.y) / 2.0 - off;
            Some(Stretch { from: Point2::new(pa.x, y), to: Point2::new(pb.x, y) })
        }
        Constraint::Distance { a, b, off, axis: 2, .. } => {
            let (pa, pb) = (at(a)?, at(b)?);
            let x = (pa.x + pb.x) / 2.0 + off;
            Some(Stretch { from: Point2::new(x, pa.y), to: Point2::new(x, pb.y) })
        }
        Constraint::DistancePL { p, a, b, off, .. } => {
            let q = at(p)?;
            let line = Side { a, b, from: at(a)?, to: at(b)? };
            let u = line.dir();
            let t = (q.x - line.from.x) * u[0] + (q.y - line.from.y) * u[1];
            let foot = Point2::new(line.from.x + u[0] * t, line.from.y + u[1] * t);
            Some(Stretch { from: Point2::new(q.x + u[0] * off, q.y + u[1] * off), to: Point2::new(foot.x + u[0] * off, foot.y + u[1] * off) })
        }
        _ => None,
    }
}

impl Project {
    /// What the place lines of the drawing `ents` of sketch `si` keep clear of: every other curve of the sketch, and
    /// every dimension line standing but the distances from a line laid from constraint `laid` on, still to be stood.
    pub(super) fn around(&self, si: usize, ents: &[Id], laid: usize) -> Around {
        let Some(s) = self.sketches.get(si) else { return Around { sides: Vec::new(), rings: Vec::new(), lines: Vec::new(), room: 0.0 } };
        let others: Vec<Id> = s.entities.iter().map(|e| e.id).filter(|id| !ents.contains(id)).collect();
        let c = self.curves_of(si, &others);
        let sides = c.sides.iter().map(|q| Stretch { from: q.from, to: q.to }).collect();
        let rings = c.rounds.iter().map(|r| Ring { at: r.at, r: r.r }).collect();
        let standing = |ci: usize, c: &Constraint| ci < laid || !matches!(c, Constraint::DistancePL { .. });
        let lines = s.constraints.iter().enumerate().filter(|(ci, c)| standing(*ci, c)).filter_map(|(_, c)| dim_line(s, c)).collect();
        Around { sides, rings, lines, room: 0.0 }
    }

    /// THE DISTANCES FROM A LINE OF SKETCH `si` FROM CONSTRAINT `laid` ON STAND BESIDE THE DRAWING THAT FILLS `bounds`:
    /// shifted along their line past it, a step at a time, the nearer side first, to where they keep clear of
    /// `around`; each one stood is kept clear of in turn. A line off the axes of the sketch is left as it falls.
    pub(super) fn stand_places_beside(&mut self, si: usize, laid: std::ops::RangeFrom<usize>, bounds: &Bounds, mut around: Around) {
        let step = bounds.step();
        around.room = step * 0.5;
        let Some(s) = self.sketches.get_mut(si) else { return };
        for ci in laid.start..s.constraints.len() {
            let Constraint::DistancePL { p, a, b, .. } = s.constraints[ci] else { continue };
            let at = |id: Id| s.points.iter().find(|q| q.id == id).map(|q| Point2::new(q.x, q.y));
            let (Some(q), Some(from), Some(to)) = (at(p), at(a), at(b)) else { continue };
            let line = Side { a, b, from, to };
            if !line.square(TURN) {
                continue;
            }
            let u = line.dir();
            let along = |w: Point2| (w.x - from.x) * u[0] + (w.y - from.y) * u[1];
            let corners = [bounds.lo, bounds.hi, Point2::new(bounds.lo.x, bounds.hi.y), Point2::new(bounds.hi.x, bounds.lo.y)];
            let lo = corners.iter().map(|w| along(*w)).fold(f64::INFINITY, f64::min);
            let hi = corners.iter().map(|w| along(*w)).fold(f64::NEG_INFINITY, f64::max);
            let tq = along(q);
            let near = if tq - lo <= hi - tq { -1.0 } else { 1.0 };
            let far = -near;
            let edge = |side: f64| if side < 0.0 { lo } else { hi };
            let tries = (1..=TRIES).flat_map(|k| [near, far].map(|side| edge(side) + side * step * k as f64 - tq));
            let mut first = None;
            let mut stood = None;
            for off in tries {
                let Some(line) = dim_line(s, &Constraint::DistancePL { p, a, b, d: 0.0, off, expr: String::new(), driven: false, at: None }) else { break };
                first.get_or_insert(Stood { off, line });
                if around.clear(line) {
                    stood = Some(Stood { off, line });
                    break;
                }
            }
            let Some(Stood { off, line }) = stood.or(first) else { continue };
            if let Constraint::DistancePL { off: o, .. } = &mut s.constraints[ci] {
                *o = off;
            }
            around.lines.push(line);
        }
    }
}
