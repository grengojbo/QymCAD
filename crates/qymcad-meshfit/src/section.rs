//! A SECTION OF A MESH FITTED WITH LINES, ARCS AND CIRCLES.
//!
//! A plane cuts a mesh in closed polygons (`qymcad_core::geom::mesh_section_loops`). Where the mesh followed a curve,
//! the corners of the polygon lie on that curve - a mesh is made by putting vertices on the surface - so the curve is
//! found from the corners, not from the chords between them. A loop is taken apart where it turns sharply, each run
//! between is reached as far as one straight line or one arc holds its corners within the tolerance, pieces that hold
//! together are joined again, and three arcs or more in a row running on into one another - a curve no line and no arc
//! fits, as an ellipse a cylinder is cut in at a slant - are one spline through their corners.

use nalgebra::{Matrix3, Vector3};
use qymcad_core::geom::Point2;

/// A corner turning more than this parts two pieces whatever they fit: 50 deg. A circle drawn in 8 chords turns 45 deg at
/// each corner and stays one circle; the corner of a rectangle turns 90.
pub const CORNER_DEG: f64 = 50.0;

/// A join of two pieces turned less than this is smooth - the line runs on into the arc along its tangent: 15 deg. The
/// tangent of an arc fitted to a shaken mesh is off at its ends: measured up to 10.8 deg on a rounding R3 drawn in 16
/// chords and shaken by 0.02, 5.7 deg on one drawn in 3 chords.
pub const SMOOTH_DEG: f64 = 15.0;

/// The fewest chords an arc is drawn by: three. Through three points goes any circle, so two chords prove nothing.
pub const ARC_CHORDS: usize = 3;

/// At least this many arcs in a row, each running on into the next along its tangent, are a free curve and one spline.
pub const SPLINE_PIECES: usize = 3;

/// A line of at most this many chords is short: joined smoothly to a free curve, it is part of the curve.
const SHORT_CHORDS: usize = 5;

/// The steps round an arc a mesh draws differ by no more than this: 4 times.
const UNEVEN: f64 = 4.0;

/// The least tolerance: 0.01 mm, what an STL written with six digits keeps of a part a metre across.
pub const LEAST_TOL: f64 = 0.01;

/// The tolerance as a share of the size of the loop: 2e-4, 0.01 mm on a part 50 mm across.
const TOL_SHARE: f64 = 2e-4;

/// HOW A LOOP IS FITTED: how far a corner may lie off the line or the arc it is taken into, and the turn that parts two
/// pieces.
#[derive(Clone, Copy, Debug)]
pub struct Fit {
    pub tol: f64,
    pub corner_deg: f64,
}

impl Fit {
    /// The fit a loop gives by itself: a tolerance of 2e-4 of its size, never under 0.01 mm. The corners of a clean mesh
    /// lie on its curves to the digits it was written with; a mesh shaken more is fitted with a tolerance typed.
    pub fn for_loop(points: &[Point2]) -> Fit {
        let (lo, hi) = points
            .iter()
            .fold((Point2::new(f64::MAX, f64::MAX), Point2::new(f64::MIN, f64::MIN)), |(lo, hi), p| (Point2::new(lo.x.min(p.x), lo.y.min(p.y)), Point2::new(hi.x.max(p.x), hi.y.max(p.y))));
        let size = if points.is_empty() { 0.0 } else { lo.dist(hi) };
        Fit { tol: (size * TOL_SHARE).max(LEAST_TOL), corner_deg: CORNER_DEG }
    }
}

/// Which way an arc turns from its start to its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    Ccw,
    Cw,
}

/// A PIECE OF A FITTED LOOP, in the coordinates of the section.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    Line {
        from: Point2,
        to: Point2,
    },
    Arc {
        center: Point2,
        radius: f64,
        from: Point2,
        to: Point2,
        turn: Turn,
    },
    /// The whole loop on one circle.
    Circle {
        center: Point2,
        radius: f64,
    },
    /// Through its points; closed when the first and the last are one.
    Spline(Vec<Point2>),
}

impl Piece {
    /// Where the piece begins; a circle has no beginning.
    pub fn start(&self) -> Option<Point2> {
        match self {
            Piece::Line { from, .. } | Piece::Arc { from, .. } => Some(*from),
            Piece::Spline(p) => p.first().copied(),
            Piece::Circle { .. } => None,
        }
    }

    /// Where the piece ends; a circle has no end.
    pub fn end(&self) -> Option<Point2> {
        match self {
            Piece::Line { to, .. } | Piece::Arc { to, .. } => Some(*to),
            Piece::Spline(p) => p.last().copied(),
            Piece::Circle { .. } => None,
        }
    }

    /// The kind of the piece, in a word, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Piece::Line { .. } => "line",
            Piece::Arc { .. } => "arc",
            Piece::Circle { .. } => "circle",
            Piece::Spline(_) => "spline",
        }
    }
}

/// How two pieces meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    /// At an angle.
    Corner,
    /// Along one tangent.
    Smooth,
}

/// A LOOP FITTED: its pieces in order round the loop, each ending where the next begins, and how each meets the next -
/// `joins[k]` is the join of piece `k` and piece `k + 1`, the last of the last and the first.
#[derive(Clone, Debug, Default)]
pub struct Fitted {
    pub pieces: Vec<Piece>,
    pub joins: Vec<Join>,
}

/// A circle in the plane.
#[derive(Clone, Copy, Debug)]
struct Circle {
    center: Point2,
    radius: f64,
}

/// What a run of corners is taken as.
#[derive(Clone, Copy, Debug)]
enum Shape {
    Line,
    Arc { circle: Circle, turn: Turn },
    Spline,
}

/// The places of the corners a run goes from and to in the walk of a loop, the walk read round past its end.
#[derive(Clone, Copy, Debug)]
struct Reach {
    from: usize,
    to: usize,
}

/// A run of the corners of the loop, `from..=to` by their places in the loop walked from its start, and what it is.
#[derive(Clone, Copy, Debug)]
struct Span {
    from: usize,
    to: usize,
    shape: Shape,
}

/// THE PIECES A CLOSED LOOP IS MADE OF. `points` are its corners in order, the first not repeated at the end.
pub fn fit_loop(points: &[Point2], fit: &Fit) -> Fitted {
    let mut pts: Vec<Point2> = Vec::with_capacity(points.len());
    for &p in points {
        if pts.last().is_none_or(|l: &Point2| l.dist(p) > fit.tol * 1e-3) {
            pts.push(p);
        }
    }
    while pts.len() > 1 && pts[0].dist(pts[pts.len() - 1]) <= fit.tol * 1e-3 {
        pts.pop();
    }
    let n = pts.len();
    if n < 3 {
        return Fitted::default();
    }
    let turns: Vec<f64> = (0..n).map(|i| turn_at([pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]])).collect();
    let limit = fit.corner_deg.to_radians();
    // THE WHOLE LOOP ON ONE CIRCLE: no corner, every turn the same way, every point within the tolerance
    if turns.iter().all(|t| t.abs() < limit) {
        let whole: Vec<Point2> = pts.iter().chain(pts.first()).copied().collect();
        if let Some(c) = circle_of(&pts).filter(|c| deviation(&pts, *c) <= fit.tol && round_once(&whole, *c, fit.tol)) {
            return Fitted { pieces: vec![Piece::Circle { center: c.center, radius: c.radius }], joins: Vec::new() };
        }
    }
    // the loop walked from its sharpest corner, so a piece does not begin in the middle of a run that is one
    let sharpest = (0..n).max_by(|&a, &b| turns[a].abs().total_cmp(&turns[b].abs())).unwrap_or(0);
    // with no corner the walk begins where the longest chord does - on a straight side if there is one - not in the
    // middle of an arc, which it would cut into pieces too short to be arcs
    let start = if turns[sharpest].abs() >= limit {
        sharpest
    } else {
        (0..n).max_by(|&a, &b| pts[a].dist(pts[(a + 1) % n]).total_cmp(&pts[b].dist(pts[(b + 1) % n]))).unwrap_or(0)
    };
    let walk: Vec<Point2> = (0..=n).map(|k| pts[(start + k) % n]).collect();
    let corner: Vec<bool> = (0..=n).map(|k| turns[(start + k) % n].abs() >= limit).collect();
    let mut spans = greedy(&walk, &corner, fit);
    join_alike(&walk, &mut spans, fit);
    let begun = if corner[0] { Begun::OnCorner } else { Begun::InRun };
    if begun == Begun::InRun {
        join_round(&walk, &mut spans, fit);
    }
    free_curves(&walk, &mut spans, begun);
    finish(&walk, &spans)
}

/// The turn at the middle of three corners in order, radians, positive to the left.
fn turn_at([a, b, c]: [Point2; 3]) -> f64 {
    let (u, v) = ((b.x - a.x, b.y - a.y), (c.x - b.x, c.y - b.y));
    (u.0 * v.1 - u.1 * v.0).atan2(u.0 * v.0 + u.1 * v.1)
}

/// Whether the closed `whole` (its first corner repeated at its end) goes round `c` once, one way.
fn round_once(whole: &[Point2], c: Circle, tol: f64) -> bool {
    let half = whole.len() / 2;
    going_round(&whole[..=half], c, tol).is_some() && going_round(&whole[half..], c, tol) == going_round(&whole[..=half], c, tol)
}

/// How far the farthest of `pts` lies off circle `c`.
fn deviation(pts: &[Point2], c: Circle) -> f64 {
    pts.iter().map(|p| (p.dist(c.center) - c.radius).abs()).fold(0.0, f64::max)
}

/// How far the farthest of `pts` lies off the segment from the first to the last.
fn off_chord(pts: &[Point2]) -> f64 {
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let len = a.dist(b);
    pts.iter().map(|p| if len < 1e-12 { p.dist(a) } else { ((b.x - a.x) * (a.y - p.y) - (a.x - p.x) * (b.y - a.y)).abs() / len }).fold(0.0, f64::max)
}

/// THE CIRCLE NEAREST `pts`: the algebraic fit (Kasa) about their mean, then five steps of Gauss-Newton on the distances
/// themselves. None for points on one line.
fn circle_of(pts: &[Point2]) -> Option<Circle> {
    let n = pts.len() as f64;
    let m = Point2::new(pts.iter().map(|p| p.x).sum::<f64>() / n, pts.iter().map(|p| p.y).sum::<f64>() / n);
    let mut a = Matrix3::<f64>::zeros();
    let mut r = Vector3::<f64>::zeros();
    for p in pts {
        let (x, y) = (p.x - m.x, p.y - m.y);
        let row = Vector3::new(x, y, 1.0);
        a += row * row.transpose();
        r += row * -(x * x + y * y);
    }
    let s = a.lu().solve(&r)?;
    let (cx, cy) = (-s[0] / 2.0, -s[1] / 2.0);
    let rr = cx * cx + cy * cy - s[2];
    if !rr.is_finite() || rr <= 0.0 {
        return None;
    }
    let mut c = Circle { center: Point2::new(cx + m.x, cy + m.y), radius: rr.sqrt() };
    for _ in 0..5 {
        let mut jtj = Matrix3::<f64>::zeros();
        let mut jtr = Vector3::<f64>::zeros();
        for p in pts {
            let d = p.dist(c.center).max(1e-12);
            let row = Vector3::new(-(p.x - c.center.x) / d, -(p.y - c.center.y) / d, -1.0);
            let res = d - c.radius;
            jtj += row * row.transpose();
            jtr += row * res;
        }
        let Some(step) = jtj.lu().solve(&-jtr) else { break };
        c = Circle { center: Point2::new(c.center.x + step[0], c.center.y + step[1]), radius: c.radius + step[2] };
    }
    (c.radius.is_finite() && c.radius > 0.0).then_some(c)
}

/// The corners of the walk a reach takes in, read round past its end.
fn run_of(walk: &[Point2], reach: Reach) -> Vec<Point2> {
    (reach.from..=reach.to).map(|k| at(walk, k)).collect()
}

/// Whether `run` is one straight line.
fn line_holds(run: &[Point2], tol: f64) -> bool {
    off_chord(run) <= tol
}

/// The arc `run` lies on, if it lies on one: at least `ARC_CHORDS` chords, turning one way, within `tol`.
fn arc_holds(run: &[Point2], tol: f64) -> Option<Shape> {
    if run.len() <= ARC_CHORDS {
        return None;
    }
    let circle = circle_of(run).filter(|c| deviation(run, *c) <= tol)?;
    // a run of corners nearly straight fits a circle as wide as the world: a line holds it, not an arc
    if circle.radius > 1e4 * tol {
        return None;
    }
    let turn = going_round(run, circle, tol)?;
    Some(Shape::Arc { circle, turn })
}

/// WHICH WAY `run` GOES ROUND `circle`, if it goes one way: each corner a step on from the last about the centre, a
/// step back allowed only within what the tolerance moves a corner at that radius - a shaken mesh turns a fine arc's
/// corners either way by its noise, and read corner by corner it was no arc at all. None for a run turning back, or
/// going a whole turn round.
fn going_round(run: &[Point2], circle: Circle, tol: f64) -> Option<Turn> {
    let angle = |p: Point2| (p.y - circle.center.y).atan2(p.x - circle.center.x);
    let step = |a: Point2, b: Point2| {
        let d = angle(b) - angle(a);
        (d + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI
    };
    let steps: Vec<f64> = run.windows(2).map(|w| step(w[0], w[1])).collect();
    let total: f64 = steps.iter().sum();
    let slack = 2.0 * tol / circle.radius;
    let back = steps.iter().any(|s| s * total.signum() < -slack);
    // A MESH PUTS THE CORNERS OF AN ARC EVENLY ROUND IT: a side of 12 mm and the first corners of the rounding after it
    // fit a circle of 64 mm within 0.01, by steps of 0.19 rad and of 0.005 - no arc a mesh draws
    let (least, most) = steps.iter().map(|s| s.abs()).fold((f64::MAX, 0.0_f64), |(lo, hi), s| (lo.min(s), hi.max(s)));
    let even = most <= UNEVEN * least.max(slack);
    (!back && even && total.abs() < 2.0 * std::f64::consts::PI - 1e-9).then_some(if total >= 0.0 { Turn::Ccw } else { Turn::Cw })
}

/// THE RUNS, EACH REACHED AS FAR AS IT GOES: from where the last ended, as far as one line holds and as far as one arc
/// holds, never past a corner; the arc where it reaches farther, the line otherwise.
fn greedy(walk: &[Point2], corner: &[bool], fit: &Fit) -> Vec<Span> {
    let last = walk.len() - 1;
    let mut spans = Vec::new();
    let mut from = 0;
    while from < last {
        // the farthest a run from `from` may go: the next corner
        let wall = (from + 1..=last).find(|&k| corner[k] || k == last).unwrap_or(last);
        let mut line_to = from + 1;
        while line_to < wall && line_holds(&walk[from..=line_to + 1], fit.tol) {
            line_to += 1;
        }
        let mut arc: Option<Span> = None;
        let mut to = from + ARC_CHORDS;
        while to <= wall {
            match arc_holds(&walk[from..=to], fit.tol) {
                Some(shape) => arc = Some(Span { from, to, shape }),
                None if arc.is_some() => break,
                None => {}
            }
            to += 1;
        }
        let span = match arc {
            Some(a) if a.to > line_to => a,
            _ => Span { from, to: line_to, shape: Shape::Line },
        };
        from = span.to;
        spans.push(span);
    }
    spans
}

/// The shape the corners of `reach` hold as one piece of the same kind as `like`, if they do.
fn refit(walk: &[Point2], reach: Reach, like: Shape, tol: f64) -> Option<Shape> {
    let run = run_of(walk, reach);
    match like {
        Shape::Line => line_holds(&run, tol).then_some(Shape::Line),
        Shape::Arc { .. } => arc_holds(&run, tol),
        Shape::Spline => None,
    }
}

/// NEIGHBOURS THAT ARE ONE: two lines on one line, two arcs on one circle, joined while any pair is.
fn join_alike(walk: &[Point2], spans: &mut Vec<Span>, fit: &Fit) {
    let mut k = 0;
    while k + 1 < spans.len() {
        let (a, b) = (spans[k], spans[k + 1]);
        let same = matches!((a.shape, b.shape), (Shape::Line, Shape::Line) | (Shape::Arc { .. }, Shape::Arc { .. }));
        match refit(walk, Reach { from: a.from, to: b.to }, a.shape, fit.tol).filter(|_| same) {
            Some(shape) => {
                spans[k] = Span { from: a.from, to: b.to, shape };
                spans.remove(k + 1);
            }
            None => k += 1,
        }
    }
}

/// THE LAST AND THE FIRST THAT ARE ONE, where the loop was not begun on a corner: the walk began inside a run, and the
/// run is cut there.
fn join_round(walk: &[Point2], spans: &mut Vec<Span>, fit: &Fit) {
    if spans.len() < 2 {
        return;
    }
    let (last, first) = (spans[spans.len() - 1], spans[0]);
    let same = matches!((last.shape, first.shape), (Shape::Line, Shape::Line) | (Shape::Arc { .. }, Shape::Arc { .. }));
    if !same {
        return;
    }
    // the two runs as one, read on past the end of the loop into its start
    let n = walk.len() - 1;
    let Some(shape) = refit(walk, Reach { from: last.from, to: first.to + n }, last.shape, fit.tol) else { return };
    spans.pop();
    spans[0] = Span { from: last.from, to: first.to + n, shape };
}

/// A FREE CURVE IS ONE SPLINE: no line and no arc fits a curve whose bending changes as it goes, so locally it falls
/// into arcs of one radius after another, each running on into the next along its tangent. Three such arcs in a row are
/// one free curve, with any short piece joined smoothly to it on either side; where every piece of the loop is taken
/// and the walk began inside a run, the whole loop is one closed spline. Arcs parted by lines - the corners of a
/// rectangle rounded - are no free curve.
fn free_curves(walk: &[Point2], spans: &mut Vec<Span>, begun: Begun) {
    let m = spans.len();
    if m < SPLINE_PIECES {
        return;
    }
    let round = begun == Begun::InRun;
    let next = |k: usize| {
        if k + 1 < m {
            Some(k + 1)
        } else if round {
            Some(0)
        } else {
            None
        }
    };
    let smooth = |a: usize, b: usize| join_of(walk, [&spans[a], &spans[b]]) == Join::Smooth;
    let arc = |k: usize| matches!(spans[k].shape, Shape::Arc { .. });
    let short = |k: usize| spans[k].to - spans[k].from <= SHORT_CHORDS || arc(k);
    let mut free = vec![false; m];
    for k in 0..m {
        let (Some(b), Some(c)) = (next(k), next(k).and_then(next)) else { continue };
        if arc(k) && arc(b) && arc(c) && smooth(k, b) && smooth(b, c) {
            free[k] = true;
            free[b] = true;
            free[c] = true;
        }
    }
    if !free.iter().any(|f| *f) {
        return;
    }
    // short pieces joined smoothly to a free curve are part of it
    loop {
        let mut grew = false;
        for k in 0..m {
            let Some(b) = next(k) else { continue };
            if free[k] && !free[b] && short(b) && smooth(k, b) || free[b] && !free[k] && short(k) && smooth(k, b) {
                free[k] = true;
                free[b] = true;
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    let n = walk.len() - 1;
    if free.iter().all(|f| *f) && round {
        let from = spans[0].from;
        *spans = vec![Span { from, to: from + n, shape: Shape::Spline }];
        return;
    }
    // the loop turned to begin after a piece that is no part of a free curve, so no run is cut by the end of the list
    let first = (0..m).find(|&k| !free[(k + m - 1) % m] && (round || k == 0)).unwrap_or(0);
    let order: Vec<usize> = (0..m).map(|k| (first + k) % m).collect();
    let mut out: Vec<Span> = Vec::with_capacity(m);
    let mut k = 0;
    while k < m {
        let i = order[k];
        if !free[i] {
            out.push(spans[i]);
            k += 1;
            continue;
        }
        let mut end = k;
        while end + 1 < m && free[order[end + 1]] {
            end += 1;
        }
        let from = spans[i].from;
        let mut to = spans[order[end]].to;
        while to < from {
            to += n;
        }
        out.push(Span { from, to, shape: Shape::Spline });
        k = end + 1;
    }
    *spans = out;
}

/// The point of the walk at place `k`, the walk read round past its end.
fn at(walk: &[Point2], k: usize) -> Point2 {
    let n = walk.len() - 1;
    walk[k % n]
}

/// Which end of a span.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Start,
    End,
}

/// Where the walk of a loop began: on a corner, or inside a run that goes on round the end of the loop.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Begun {
    OnCorner,
    InRun,
}

/// The direction a span runs in at its `side`, as a unit vector.
fn tangent(walk: &[Point2], s: &Span, side: Side) -> Point2 {
    let unit = |d: Point2| {
        let l = (d.x * d.x + d.y * d.y).sqrt().max(1e-12);
        Point2::new(d.x / l, d.y / l)
    };
    match s.shape {
        Shape::Arc { circle, turn } => {
            let p = if side == Side::End { at(walk, s.to) } else { at(walk, s.from) };
            let r = Point2::new(p.x - circle.center.x, p.y - circle.center.y);
            unit(if turn == Turn::Ccw { Point2::new(-r.y, r.x) } else { Point2::new(r.y, -r.x) })
        }
        Shape::Line => {
            let (a, b) = (at(walk, s.from), at(walk, s.to));
            unit(Point2::new(b.x - a.x, b.y - a.y))
        }
        Shape::Spline => {
            let (a, b) = if side == Side::End { (at(walk, s.to - 1), at(walk, s.to)) } else { (at(walk, s.from), at(walk, s.from + 1)) };
            unit(Point2::new(b.x - a.x, b.y - a.y))
        }
    }
}

/// How a span meets the span that follows it.
fn join_of(walk: &[Point2], [a, b]: [&Span; 2]) -> Join {
    let (u, v) = (tangent(walk, a, Side::End), tangent(walk, b, Side::Start));
    let angle = (u.x * v.y - u.y * v.x).atan2(u.x * v.x + u.y * v.y).abs();
    if angle <= SMOOTH_DEG.to_radians() {
        Join::Smooth
    } else {
        Join::Corner
    }
}

/// THE PIECES, their ends closed up: where an arc meets a line or a spline, the corner they share is put on the arc's
/// circle and both end there; where two arcs meet, on the circle of the first.
fn finish(walk: &[Point2], spans: &[Span]) -> Fitted {
    let m = spans.len();
    let on_circle = |p: Point2, c: Circle| {
        let d = p.dist(c.center).max(1e-12);
        Point2::new(c.center.x + (p.x - c.center.x) * c.radius / d, c.center.y + (p.y - c.center.y) * c.radius / d)
    };
    // the point where span k ends and span k + 1 begins
    let shared: Vec<Point2> = (0..m)
        .map(|k| {
            let p = at(walk, spans[k].to);
            match (spans[k].shape, spans[(k + 1) % m].shape) {
                (Shape::Arc { circle, .. }, _) | (_, Shape::Arc { circle, .. }) => on_circle(p, circle),
                _ => p,
            }
        })
        .collect();
    let pieces = (0..m)
        .map(|k| {
            let (from, to) = (shared[(k + m - 1) % m], shared[k]);
            let s = &spans[k];
            match s.shape {
                Shape::Line => Piece::Line { from, to },
                Shape::Arc { circle, turn } => Piece::Arc { center: circle.center, radius: circle.radius, from, to, turn },
                Shape::Spline => {
                    let mut p: Vec<Point2> = (s.from..=s.to).map(|i| at(walk, i)).collect();
                    if let Some(f) = p.first_mut() {
                        *f = from;
                    }
                    if let Some(l) = p.last_mut() {
                        *l = to;
                    }
                    Piece::Spline(p)
                }
            }
        })
        .collect();
    let joins = (0..m).map(|k| join_of(walk, [&spans[k], &spans[(k + 1) % m]])).collect();
    Fitted { pieces, joins }
}
