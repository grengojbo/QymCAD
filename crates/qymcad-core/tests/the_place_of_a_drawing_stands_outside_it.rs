//! THE PLACE OF A DRAWING IS MEASURED OUTSIDE IT, as on a drawing on paper: a distance from an axis or from the edge of
//! the face a drawing stands on runs beside the drawing, clear of every curve of the sketch, never along a side of the
//! drawing nor across another outline; and no size says what another already says.
//!
//! Reported behaviour: Fully define laid the distance of a circle from the Y axis straight across the rectangle beside
//! it; Sketch from face laid the distance 30.0 from the edge of the plate along the left side of the boss; Sketch from
//! section laid the place of a hole inside the plate and the height 30.0 of the plate twice, once on its left side.
use qymcad_core::feature::Purpose;
use qymcad_core::geom::Point2;
use qymcad_core::model::{Anchor, Constraint, Id, Project};

fn polygon(p: &mut Project, si: usize, pts: &[[f64; 2]], purpose: Purpose) -> Vec<Id> {
    (0..pts.len())
        .map(|k| {
            let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
            p.add_line_entity(si, a[0], a[1], b[0], b[1], purpose)
        })
        .collect()
}

/// A stretch of a line: where it starts and where it ends.
#[derive(Clone, Copy, Debug)]
struct Stretch {
    from: Point2,
    to: Point2,
}

impl Stretch {
    /// Which side of the stretch `q` lies on, scaled by its length: the sign of the turn from it to `q`.
    fn side_of(self, q: Point2) -> f64 {
        (self.to.x - self.from.x) * (q.y - self.from.y) - (self.to.y - self.from.y) * (q.x - self.from.x)
    }

    /// Whether this stretch and `o` cross or touch.
    fn meets(self, o: Stretch) -> bool {
        if o.side_of(self.from) * o.side_of(self.to) < 0.0 && self.side_of(o.from) * self.side_of(o.to) < 0.0 {
            return true;
        }
        [to_stretch(self.from, o), to_stretch(self.to, o), to_stretch(o.from, self), to_stretch(o.to, self)].iter().any(|d| *d < 1e-6)
    }
}

/// A circle of the sketch: its centre and radius.
struct Ring {
    c: Point2,
    r: f64,
}

/// What a dimension line may not touch: the sides and the circles of the sketch, construction left out.
struct Curves {
    sides: Vec<Stretch>,
    rings: Vec<Ring>,
}

fn curves(p: &Project, si: usize) -> Curves {
    let s = &p.sketches[si];
    let at = |id: Id| s.points.iter().find(|q| q.id == id).map(|q| Point2::new(q.x, q.y));
    let mut sides = Vec::new();
    let mut rings = Vec::new();
    for e in s.entities.iter().filter(|e| !e.construction) {
        if let Some(l) = e.kind.as_line() {
            if let (Some(from), Some(to)) = (at(l.a), at(l.b)) {
                sides.push(Stretch { from, to });
            }
        }
        if let (Some(circle), Some(c)) = (e.kind.circle_by(&at), e.kind.round_centre().and_then(at)) {
            rings.push(Ring { c, r: circle.r });
        }
    }
    Curves { sides, rings }
}

/// The distance from `q` to the stretch `s`.
fn to_stretch(q: Point2, s: Stretch) -> f64 {
    let (dx, dy) = (s.to.x - s.from.x, s.to.y - s.from.y);
    let l2 = (dx * dx + dy * dy).max(1e-18);
    let t = (((q.x - s.from.x) * dx + (q.y - s.from.y) * dy) / l2).clamp(0.0, 1.0);
    (q.x - s.from.x - t * dx).hypot(q.y - s.from.y - t * dy)
}

/// The dimension lines of the distances from a line among the constraints `laid`, as they are drawn: the
/// perpendicular from the point to its foot on the line, shifted along the line by `off`.
fn place_lines(p: &Project, si: usize, laid: std::ops::RangeFrom<usize>) -> Vec<Stretch> {
    let s = &p.sketches[si];
    let at = |id: Id| s.points.iter().find(|q| q.id == id).map(|q| Point2::new(q.x, q.y));
    s.constraints[laid]
        .iter()
        .filter_map(|c| {
            let Constraint::DistancePL { p, a, b, off, .. } = *c else { return None };
            let (q, a, b) = (at(p)?, at(a)?, at(b)?);
            let l = (b.x - a.x).hypot(b.y - a.y).max(1e-12);
            let u = [(b.x - a.x) / l, (b.y - a.y) / l];
            let t = (q.x - a.x) * u[0] + (q.y - a.y) * u[1];
            let foot = Point2::new(a.x + u[0] * t, a.y + u[1] * t);
            Some(Stretch { from: Point2::new(q.x + u[0] * off, q.y + u[1] * off), to: Point2::new(foot.x + u[0] * off, foot.y + u[1] * off) })
        })
        .collect()
}

/// Every distance from a line among the constraints `laid` runs clear of every curve of the sketch.
fn clear(p: &Project, si: usize, laid: std::ops::RangeFrom<usize>) -> Vec<String> {
    let c = curves(p, si);
    let mut wrong = Vec::new();
    for l in place_lines(p, si, laid) {
        if let Some(side) = c.sides.iter().find(|s| l.meets(**s)) {
            wrong.push(format!("the place line {l:?} meets the side {side:?}"));
        }
        let near = |r: &Ring| to_stretch(r.c, l) <= r.r + 1e-6 && (l.from.x - r.c.x).hypot(l.from.y - r.c.y).max((l.to.x - r.c.x).hypot(l.to.y - r.c.y)) >= r.r - 1e-6;
        if c.rings.iter().any(near) {
            wrong.push(format!("the place line {l:?} crosses a circle"));
        }
    }
    wrong
}

/// THE LAYOUT OF THE MANUAL CHECK OF FULLY DEFINE: a rectangle and, to its right, a circle, each defined by itself from
/// the axes. The distance of the circle from the Y axis runs past the rectangle, not across it.
#[test]
fn the_place_of_a_circle_runs_past_the_rectangle_beside_it() {
    let mut p = Project::default();
    let si = p.new_sketch("S");
    let rect = polygon(&mut p, si, &[[10.0, 10.0], [50.0, 10.0], [50.0, 40.0], [10.0, 40.0]], Purpose::Real);
    p.fully_define(si, &rect, &Anchor::Origin);
    let from = p.sketches[si].constraints.len();
    let circle = vec![p.add_circle_entity(si, 75.0, 25.0, 7.0, Purpose::Real)];
    p.fully_define(si, &circle, &Anchor::Origin);
    assert_eq!(p.sketch_dof(si), (0, 0));
    let wrong = clear(&p, si, from..);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// THE LAYOUT OF THE MANUAL CHECK OF SKETCH FROM FACE: a boss of 30 x 20 on a plate of 60 x 40, placed from the edges of
/// the plate held as construction. The distances from the edges run between the edge and the boss, not along a side
/// of the boss.
#[test]
fn the_place_of_a_boss_runs_between_the_edge_and_the_boss() {
    let mut p = Project::default();
    let si = p.new_sketch("S");
    let plate = polygon(&mut p, si, &[[0.0, 0.0], [60.0, 0.0], [60.0, 40.0], [0.0, 40.0]], Purpose::Construction);
    let held: std::collections::BTreeSet<Id> = p.sketches[si].entities.iter().filter(|e| plate.contains(&e.id)).flat_map(qymcad_core::model::entity_points).collect();
    for q in held {
        p.sketches[si].constraints.push(Constraint::Fixed { p: q });
    }
    let boss = polygon(&mut p, si, &[[10.0, 10.0], [40.0, 10.0], [40.0, 30.0], [10.0, 30.0]], Purpose::Real);
    let from = p.sketches[si].constraints.len();
    p.fully_define(si, &boss, &Anchor::Lines(plate));
    assert_eq!(p.sketch_dof(si), (0, 0));
    let mut wrong = clear(&p, si, from..);
    // between the edge and the boss: off the boss along the edge, within the plate
    for l in place_lines(&p, si, from..) {
        let x = l.from.x;
        let y = l.from.y;
        let vertical = (l.from.x - l.to.x).abs() < 1e-9;
        let off_the_boss = if vertical { !(10.0..=40.0).contains(&x) } else { !(10.0..=30.0).contains(&y) };
        if !off_the_boss {
            wrong.push(format!("the place line {l:?} stands within the boss"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// THE LAYOUT OF THE MANUAL CHECK OF SKETCH FROM SECTION: a plate of 40 x 30 from the origin with a hole of 10 at
/// (20, 15). The place of the hole stands outside the plate, and the height 30 is laid once.
#[test]
fn the_place_of_a_hole_stands_outside_the_plate_and_no_size_is_laid_twice() {
    let mut p = Project::default();
    let si = p.new_sketch("S");
    // drawn from the top left corner, as a section is walked: the first corner met stands 30 above the X axis
    let mut ents = polygon(&mut p, si, &[[0.0, 30.0], [0.0, 0.0], [40.0, 0.0], [40.0, 30.0]], Purpose::Real);
    ents.push(p.add_circle_entity(si, 20.0, 15.0, 5.0, Purpose::Real));
    let from = p.sketches[si].constraints.len();
    p.fully_define(si, &ents, &Anchor::Origin);
    assert_eq!(p.sketch_dof(si), (0, 0));
    let mut wrong = clear(&p, si, from..);
    for l in place_lines(&p, si, from..) {
        let inside = |q: Point2| q.x > 1e-6 && q.x < 40.0 - 1e-6 && q.y > 1e-6 && q.y < 30.0 - 1e-6;
        if inside(l.from) || inside(l.to) {
            wrong.push(format!("the place line {l:?} stands inside the plate"));
        }
    }
    let thirty = p.sketches[si].constraints[from..]
        .iter()
        .filter(|c| match c {
            Constraint::Distance { d, .. } | Constraint::DistancePL { d, .. } => (d.abs() - 30.0).abs() < 1e-9,
            _ => false,
        })
        .count();
    if thirty != 1 {
        wrong.push(format!("the size 30 is laid {thirty} times"));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
