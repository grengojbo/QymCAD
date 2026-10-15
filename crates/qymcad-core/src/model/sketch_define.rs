//! A DRAWING DEFINED TO THE END: loose curves given the relations they show and then sizes, until no freedom is left.
//!
//! A face copied into a sketch comes as loose curves: right where they stand, held by nothing. Edited, a side dragged
//! takes its corner off square and a size typed moves whatever is free. What a person would lay by hand is laid
//! instead: first the relations the curves already show - horizontal, vertical, parallel, perpendicular, tangent,
//! concentric, equal - then sizes, each kept only when it takes a freedom away (`solver::freedom_taken`), in the order a
//! drawing is read: radii, lengths, distances between parallel sides, angles, the place, the turn. The sizes are then
//! rounded to a tenth: a file that says 29.97 meant 30.
use super::{Constraint, EntityKind, Id, Project};
use crate::geom::Point2;
use std::collections::HashMap;

/// How far off a relation may stand and still be taken for one, in radians: 0.01 deg. A face of an exact solid is true
/// to its tolerance (1e-7 mm over 100 mm is 1e-9 rad); a side drawn by hand 0.5 deg off square is not taken for square.
const TURN: f64 = 1.75e-4;
/// Two lengths or two radii closer than this, in mm, are taken for equal; two points this close for one.
const SAME: f64 = 1e-6;
/// The step the sizes are rounded to: a tenth of a millimetre, a tenth of a degree.
const TENTH: f64 = 0.1;
/// A solve leaving more than this of the rounded sizes unmet failed them: the sizes as they were are put back.
const MET: f64 = 1e-6;

/// WHAT A DRAWING IS PLACED FROM: the axes of its sketch, or lines already held - the outline of the face it stands on.
#[derive(Clone, Debug)]
pub enum Anchor {
    Origin,
    Lines(Vec<Id>),
}

/// WHAT `fully_define` LAID: how many relations and sizes went on, the freedoms left, and each size rounded.
#[derive(Clone, Debug, Default)]
pub struct Defined {
    pub relations: usize,
    pub sizes: usize,
    pub free: i32,
    pub rounded: Vec<Rounded>,
}

/// A size as it was measured and as it was laid.
#[derive(Clone, Copy, Debug)]
pub struct Rounded {
    pub was: f64,
    pub now: f64,
}

/// A straight side: its two ends by id and where they stand.
struct Side {
    a: Id,
    b: Id,
    from: Point2,
    to: Point2,
}

impl Side {
    fn len(&self) -> f64 {
        (self.to.x - self.from.x).hypot(self.to.y - self.from.y)
    }

    /// The direction from `a` to `b`, of length one.
    fn dir(&self) -> [f64; 2] {
        let l = self.len().max(1e-12);
        [(self.to.x - self.from.x) / l, (self.to.y - self.from.y) / l]
    }

    fn ends(&self) -> [Id; 2] {
        [self.a, self.b]
    }

    /// The other end than `p`.
    fn far(&self, p: Id) -> Id {
        if self.a == p {
            self.b
        } else {
            self.a
        }
    }

    /// The signed distance of `p` from the line through the side, as `Constraint::DistancePL` measures it.
    fn off(&self, p: Point2) -> f64 {
        let d = self.dir();
        d[0] * (p.y - self.from.y) - d[1] * (p.x - self.from.x)
    }

    /// Whether the side runs along the X or the Y axis of the sketch.
    fn square(&self) -> bool {
        let d = self.dir();
        d[0].abs() < TURN || d[1].abs() < TURN
    }
}

/// A circle or an arc: its centre by id and where it stands, its radius, and the ends of an arc.
struct Round {
    centre: Id,
    at: Point2,
    r: f64,
    ends: Option<[Id; 2]>,
}

/// The curves of a drawing, read off its sketch.
struct Curves {
    sides: Vec<Side>,
    rounds: Vec<Round>,
    at: HashMap<Id, Point2>,
}

/// The angle between two directions of length one, in degrees, from 0 to 180.
fn angle(u: [f64; 2], v: [f64; 2]) -> f64 {
    (u[0] * v[1] - u[1] * v[0]).abs().atan2(u[0] * v[0] + u[1] * v[1]).to_degrees()
}

fn dot(u: [f64; 2], v: [f64; 2]) -> f64 {
    u[0] * v[0] + u[1] * v[1]
}

fn cross(u: [f64; 2], v: [f64; 2]) -> f64 {
    u[0] * v[1] - u[1] * v[0]
}

/// The direction from `c` to `p`, of length one.
fn radial(c: Point2, p: Point2) -> [f64; 2] {
    let l = (p.x - c.x).hypot(p.y - c.y).max(1e-12);
    [(p.x - c.x) / l, (p.y - c.y) / l]
}

/// THE RELATIONS THE CURVES SHOW, in the order they are offered: a side along an axis, then sides parallel and sides
/// meeting square, a side running into an arc along it, arcs running into each other, circles on one centre and of one
/// radius, sides of one length, and ends standing on one place under two names.
fn relations(c: &Curves) -> Vec<Constraint> {
    let mut out = Vec::new();
    for s in &c.sides {
        let d = s.dir();
        if d[1].abs() < TURN {
            out.push(Constraint::Horizontal { a: s.a, b: s.b });
        } else if d[0].abs() < TURN {
            out.push(Constraint::Vertical { a: s.a, b: s.b });
        }
    }
    for (i, s) in c.sides.iter().enumerate() {
        for t in &c.sides[i + 1..] {
            if s.square() && t.square() {
                continue; // along the axes both: the relation follows from the two
            }
            let shared = s.ends().iter().any(|p| t.ends().contains(p));
            if cross(s.dir(), t.dir()).abs() < TURN {
                out.push(Constraint::Parallel { a: s.a, b: s.b, c: t.a, d: t.b });
            } else if shared && dot(s.dir(), t.dir()).abs() < TURN {
                out.push(Constraint::Perpendicular { a: s.a, b: s.b, c: t.a, d: t.b });
            }
        }
    }
    for r in &c.rounds {
        let Some(ends) = r.ends else { continue };
        for p in ends {
            let Some(&at) = c.at.get(&p) else { continue };
            for s in c.sides.iter().filter(|s| s.ends().contains(&p)) {
                if dot(s.dir(), radial(r.at, at)).abs() < TURN {
                    out.push(Constraint::Tangent { a: s.a, b: s.b, c: r.centre, r: r.r });
                }
            }
        }
    }
    for (i, r) in c.rounds.iter().enumerate() {
        for q in &c.rounds[..i] {
            let (Some(re), Some(qe)) = (r.ends, q.ends) else { continue };
            for p in re.iter().filter(|p| qe.contains(p)) {
                let Some(&at) = c.at.get(p) else { continue };
                let (u, v) = (radial(r.at, at), radial(q.at, at));
                if cross(u, v).abs() < TURN {
                    out.push(Constraint::CircleTangent { c1: q.centre, c2: r.centre, external: dot(u, v) < 0.0 });
                }
            }
        }
    }
    for (i, r) in c.rounds.iter().enumerate() {
        if let Some(q) = c.rounds[..i].iter().find(|q| (q.at.x - r.at.x).hypot(q.at.y - r.at.y) < SAME && q.centre != r.centre) {
            out.push(Constraint::Concentric { c1: q.centre, c2: r.centre });
        }
        if let Some(q) = c.rounds[..i].iter().find(|q| (q.r - r.r).abs() < SAME) {
            out.push(Constraint::EqualRadius { c1: q.centre, c2: r.centre });
        }
    }
    // sides of one length last: around arcs held tangent and of one radius, the length follows from them
    for (i, s) in c.sides.iter().enumerate() {
        if let Some(t) = c.sides[..i].iter().find(|t| (t.len() - s.len()).abs() < SAME) {
            out.push(Constraint::Equal { a: t.a, b: t.b, c: s.a, d: s.b });
        }
    }
    let mut ends: Vec<Id> = c.sides.iter().flat_map(Side::ends).chain(c.rounds.iter().filter_map(|r| r.ends).flatten()).collect();
    ends.sort_unstable();
    ends.dedup();
    for (i, p) in ends.iter().enumerate() {
        let Some(&a) = c.at.get(p) else { continue };
        if let Some(q) = ends[..i].iter().find(|q| c.at.get(q).is_some_and(|b| (a.x - b.x).hypot(a.y - b.y) < SAME)) {
            out.push(Constraint::Coincident { a: *q, b: *p });
        }
    }
    out
}

/// A size in its plain form: driving, no expression, its label where it falls.
fn distance(a: Id, b: Id, d: f64, axis: u8) -> Constraint {
    Constraint::Distance { a, b, d, off: 0.0, expr: String::new(), driven: false, axis, at: None }
}

/// A point at its distance from a line; a point standing on the line is laid on it, not 0 from it.
fn to_line(p: Id, a: Id, b: Id, d: f64) -> Constraint {
    if d.abs() < SAME {
        return Constraint::PointOnLine { p, a, b };
    }
    Constraint::DistancePL { p, a, b, d, off: 0.0, expr: String::new(), driven: false, at: None }
}

/// THE SIZES OFFERED, in the order a drawing is read: radii and diameters, distances between parallel sides, lengths
/// of sides, angles where sides meet, then the place - each point from each line it is anchored to - and the
/// turn of each side against them.
fn sizes(c: &Curves, anchors: &[Side]) -> Vec<Constraint> {
    let mut out = Vec::new();
    for r in &c.rounds {
        let (d, diam) = if r.ends.is_some() { (r.r, false) } else { (2.0 * r.r, true) };
        out.push(Constraint::Diameter { c: r.centre, d, off: 0.0, expr: String::new(), driven: false, diam, at: None });
    }
    // the span between parallel sides before the length of a side: over rounded corners the span is the size of the
    // part, the straight run of a side only what the corners leave
    for (i, s) in c.sides.iter().enumerate() {
        for t in &c.sides[i + 1..] {
            let shared = s.ends().iter().any(|p| t.ends().contains(p));
            if shared || cross(s.dir(), t.dir()).abs() >= TURN {
                continue;
            }
            // sides along an axis are measured along the other axis, end to end. The distance of a point from a side
            // that may turn is read by the rank with the turn in it, and two sizes saying one thing both counted: an L of
            // six sides took its width twice and was left free to slide along X, a rectangle turned 30 deg gave 0.3 mm
            // under a pull. Turned sides are left to their lengths.
            let d = s.dir();
            if d[1].abs() < TURN {
                out.push(distance(s.a, t.a, (t.from.y - s.from.y).abs(), 2));
            } else if d[0].abs() < TURN {
                out.push(distance(s.a, t.a, (t.from.x - s.from.x).abs(), 1));
            }
        }
    }
    for s in &c.sides {
        let d = s.dir();
        let axis = if d[1].abs() < TURN {
            1
        } else if d[0].abs() < TURN {
            2
        } else {
            0
        };
        let len = if axis == 1 {
            (s.to.x - s.from.x).abs()
        } else if axis == 2 {
            (s.to.y - s.from.y).abs()
        } else {
            s.len()
        };
        out.push(distance(s.a, s.b, len, axis));
    }
    for (i, s) in c.sides.iter().enumerate() {
        for t in &c.sides[i + 1..] {
            let Some(&p) = s.ends().iter().find(|p| t.ends().contains(p)) else { continue };
            let Some(&at) = c.at.get(&p) else { continue };
            let (fa, fb) = (s.far(p), t.far(p));
            let (Some(&a), Some(&b)) = (c.at.get(&fa), c.at.get(&fb)) else { continue };
            let deg = angle(radial(at, a), radial(at, b));
            if deg.abs() > TURN.to_degrees() && (deg - 180.0).abs() > TURN.to_degrees() {
                out.push(Constraint::Angle { a: fa, b: p, c: fb, deg, expr: String::new(), driven: false, off: 0.0, at: None });
            }
        }
    }
    let mut points: Vec<Id> = c.rounds.iter().map(|r| r.centre).collect();
    points.extend(c.sides.iter().flat_map(Side::ends));
    let mut seen = std::collections::HashSet::new();
    points.retain(|p| seen.insert(*p));
    // the first point placed, then the turn of the sides, then the other points: a drawing turned is told by its angle,
    // not by the place of a second corner
    let place = |p: Id, out: &mut Vec<Constraint>| {
        if let Some(&at) = c.at.get(&p) {
            out.extend(anchors.iter().map(|l| to_line(p, l.a, l.b, l.off(at))));
        }
    };
    if let Some(&first) = points.first() {
        place(first, &mut out);
    }
    for s in c.sides.iter().filter(|s| !s.square()) {
        for l in anchors {
            out.push(Constraint::AngleLines { a: l.a, b: l.b, c: s.a, d: s.b, deg: angle(l.dir(), s.dir()), expr: String::new(), driven: false, off: 0.0, at: None });
        }
    }
    for &p in points.iter().skip(1) {
        place(p, &mut out);
    }
    out
}

/// The value a size holds, to be rounded: a length, a radius, a distance or an angle.
fn size_of(c: &mut Constraint) -> Option<&mut f64> {
    match c {
        Constraint::Distance { d, .. } | Constraint::DistancePL { d, .. } | Constraint::Diameter { d, .. } => Some(d),
        Constraint::Angle { deg, .. } | Constraint::AngleLines { deg, .. } => Some(deg),
        _ => None,
    }
}

impl Project {
    /// The sides, the circles and the arcs among `ents` of sketch `si`, construction curves left out, and where every
    /// point of the sketch stands.
    fn curves_of(&self, si: usize, ents: &[Id]) -> Curves {
        let Some(s) = self.sketches.get(si) else { return Curves { sides: Vec::new(), rounds: Vec::new(), at: HashMap::new() } };
        let at: HashMap<Id, Point2> = s.points.iter().map(|p| (p.id, Point2::new(p.x, p.y))).collect();
        let mut sides = Vec::new();
        let mut rounds = Vec::new();
        for e in s.entities.iter().filter(|e| ents.contains(&e.id) && !e.construction) {
            match e.kind {
                EntityKind::Line { a, b } => {
                    if let (Some(&from), Some(&to)) = (at.get(&a), at.get(&b)) {
                        sides.push(Side { a, b, from, to });
                    }
                }
                EntityKind::Circle { center, r } => {
                    if let Some(&c) = at.get(&center) {
                        rounds.push(Round { centre: center, at: c, r, ends: None });
                    }
                }
                EntityKind::Arc { center, a, b, .. } => {
                    if let (Some(&c), Some(&p)) = (at.get(&center), at.get(&a)) {
                        rounds.push(Round { centre: center, at: c, r: (p.x - c.x).hypot(p.y - c.y), ends: Some([a, b]) });
                    }
                }
                EntityKind::Ellipse { .. } => {}
            }
        }
        Curves { sides, rounds, at }
    }

    /// THE LINES A DRAWING IS PLACED FROM, as sides: the axes of the sketch, through its frame point and the guides of
    /// X and Y, or the straight lines named.
    fn anchor_sides(&mut self, si: usize, anchor: &Anchor) -> Vec<Side> {
        if matches!(anchor, Anchor::Origin) {
            // the axes are laid when first asked for, held where they belong
            self.ensure_axis(si, 0);
            self.ensure_axis(si, 1);
        }
        let Some(s) = self.sketches.get(si) else { return Vec::new() };
        let at = |id: Id| s.points.iter().find(|p| p.id == id).map(|p| Point2::new(p.x, p.y));
        let side = |a: Id, b: Id| Some(Side { a, b, from: at(a)?, to: at(b)? });
        match anchor {
            Anchor::Origin => s.axis_pts.iter().filter_map(|&g| side(s.frame, g)).collect(),
            Anchor::Lines(ids) => s
                .entities
                .iter()
                .filter(|e| ids.contains(&e.id))
                .filter_map(|e| match e.kind {
                    EntityKind::Line { a, b } => side(a, b),
                    _ => None,
                })
                .collect(),
        }
    }

    /// THE CANDIDATES THAT TAKE A FREEDOM AWAY, laid on sketch `si` in their order; the rest dropped. Each is laid and
    /// judged in the sketch as it then stands: what an arc holds its ends by changes with a tangency laid on it
    /// (`entity_intrinsics`), and judged against the arc as it was, a tangency read as taking nothing. A relation is
    /// judged where the constraints hold (`freedom_taken_where_solved`): two relations nearly met, each read at the
    /// geometry as it stands, both counted as taking the freedom one of them takes. A size, measured off the geometry,
    /// is met where it stands. Answers how many went on.
    fn lay_independent(&mut self, si: usize, offered: Vec<Constraint>, solved: bool) -> usize {
        let mut n = 0;
        for c in offered {
            let Some(s) = self.sketches.get_mut(si) else { return n };
            s.constraints.push(c);
            let s = &self.sketches[si];
            let mut active: Vec<Constraint> = s.constraints.iter().filter(|c| !c.is_driven()).cloned().collect();
            let k = active.len() - 1;
            active.extend(self.entity_intrinsics(si));
            let radii = self.entity_radii(si);
            let takes = if solved { crate::solver::freedom_taken_where_solved(&s.points, &radii, &active, k) } else { crate::solver::freedom_taken(&s.points, &radii, &active, k) };
            if takes > 0 {
                n += 1;
            } else {
                self.sketches[si].constraints.pop();
            }
        }
        n
    }

    /// LAY ON THE CURVES `ents` OF SKETCH `si` THE RELATIONS THEY SHOW, each that takes a freedom away. Answers how many
    /// went on.
    pub fn infer_constraints(&mut self, si: usize, ents: &[Id]) -> usize {
        let offered = relations(&self.curves_of(si, ents));
        let n = self.lay_independent(si, offered, true);
        if n > 0 {
            self.solve_sketch(si);
        }
        n
    }

    /// DEFINE THE CURVES `ents` OF SKETCH `si` TO THE END: the relations they show (`infer_constraints`), then sizes until
    /// no freedom is left, the place measured from `anchor`, the sizes rounded to a tenth. When the rounded sizes cannot
    /// all be met at once - a tangency a tenth of a millimetre would break - they are laid as measured and nothing is
    /// told rounded.
    pub fn fully_define(&mut self, si: usize, ents: &[Id], anchor: &Anchor) -> Defined {
        let relations = self.infer_constraints(si, ents);
        let anchors = self.anchor_sides(si, anchor);
        let from = self.sketches.get(si).map_or(0, |s| s.constraints.len());
        let offered = sizes(&self.curves_of(si, ents), &anchors);
        let sizes = self.lay_independent(si, offered, false);
        let rounded = self.round_sizes(si, from);
        Defined { relations, sizes, free: self.sketch_dof(si).0, rounded }
    }

    /// The sizes of sketch `si` from constraint `from` on rounded to a tenth and the sketch solved to them; put back as
    /// they were when the solve cannot meet them all. Answers each size rounded.
    fn round_sizes(&mut self, si: usize, from: usize) -> Vec<Rounded> {
        let Some(s) = self.sketches.get_mut(si) else { return Vec::new() };
        let was: Vec<Constraint> = s.constraints.clone();
        let points = s.points.clone();
        let mut rounded = Vec::new();
        for c in s.constraints.iter_mut().skip(from) {
            if let Some(d) = size_of(c) {
                let now = (*d / TENTH).round() * TENTH;
                if (now - *d).abs() > 1e-9 {
                    rounded.push(Rounded { was: *d, now });
                    *d = now;
                }
            }
        }
        if rounded.is_empty() {
            return rounded;
        }
        if self.solve_sketch(si) > MET {
            let s = &mut self.sketches[si];
            s.constraints = was;
            s.points = points;
            self.solve_sketch(si);
            return Vec::new();
        }
        rounded
    }
}
