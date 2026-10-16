//! A SECTION OF A MESH FITTED WITH LINES, ARCS AND CIRCLES: shapes drawn as a mesh draws them - the corners of a polygon
//! on the true curve, as many to a quarter of a turn as the mesh was made with - and the pieces found must be the
//! shape's own, its sizes within the tolerance. Shape x fineness x noise; the failures are gathered and told together.
use qymcad_core::geom::Point2;
use qymcad_meshfit::section::{fit_loop, Fit, Join, Piece};
use std::f64::consts::PI;

/// A shape drawn as a closed polygon, and what it must be found as.
struct Case {
    name: String,
    points: Vec<Point2>,
    lines: usize,
    arcs: usize,
    circles: usize,
    /// the radii of the arcs and circles from the smallest, each within the tolerance
    radii: Vec<f64>,
    /// how many joins of two pieces are smooth - a line running on into an arc along its tangent
    smooth: usize,
}

/// The tolerance a person types for a mesh that is not clean.
const NOISY_TOL: f64 = 0.06;

/// An arc about `center` of `radius`, from the angle `from` turned by `sweep` (radians).
struct Round {
    center: Point2,
    radius: f64,
    from: f64,
    sweep: f64,
}

/// An arc drawn as `n` chords, its end point left out.
fn arc(r: Round, n: usize) -> Vec<Point2> {
    (0..n).map(|k| r.from + r.sweep * k as f64 / n as f64).map(|a| Point2::new(r.center.x + r.radius * a.cos(), r.center.y + r.radius * a.sin())).collect()
}

/// A straight side from `a` towards `b` in `n` even steps, its end point left out - a mesh puts vertices along a long
/// side too.
fn side([a, b]: [Point2; 2], n: usize) -> Vec<Point2> {
    (0..n).map(|k| k as f64 / n as f64).map(|t| Point2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)).collect()
}

/// The width and the height of a rectangle.
struct Size {
    w: f64,
    h: f64,
}

/// A rectangle of `size` at (5, 5) with its corners rounded by `r`, a quarter in `q` chords.
fn rounded(size: Size, r: f64, q: usize) -> Vec<Point2> {
    let (x0, y0, x1, y1) = (5.0, 5.0, 5.0 + size.w, 5.0 + size.h);
    let mut p = Vec::new();
    p.extend(side([Point2::new(x0 + r, y0), Point2::new(x1 - r, y0)], 3));
    p.extend(arc(Round { center: Point2::new(x1 - r, y0 + r), radius: r, from: -PI / 2.0, sweep: PI / 2.0 }, q));
    p.extend(side([Point2::new(x1, y0 + r), Point2::new(x1, y1 - r)], 2));
    p.extend(arc(Round { center: Point2::new(x1 - r, y1 - r), radius: r, from: 0.0, sweep: PI / 2.0 }, q));
    p.extend(side([Point2::new(x1 - r, y1), Point2::new(x0 + r, y1)], 3));
    p.extend(arc(Round { center: Point2::new(x0 + r, y1 - r), radius: r, from: PI / 2.0, sweep: PI / 2.0 }, q));
    p.extend(side([Point2::new(x0, y1 - r), Point2::new(x0, y0 + r)], 2));
    p.extend(arc(Round { center: Point2::new(x0 + r, y0 + r), radius: r, from: PI, sweep: PI / 2.0 }, q));
    p
}

/// A slot: two half circles of `radius`, their centres `length` apart along X.
struct Slot {
    length: f64,
    radius: f64,
}

/// A slot drawn with `half` chords to each of its ends.
fn slot(Slot { length: l, radius: r }: Slot, half: usize) -> Vec<Point2> {
    let mut p = Vec::new();
    p.extend(side([Point2::new(0.0, -r), Point2::new(l, -r)], 2));
    p.extend(arc(Round { center: Point2::new(l, 0.0), radius: r, from: -PI / 2.0, sweep: PI }, half));
    p.extend(side([Point2::new(l, r), Point2::new(0.0, r)], 2));
    p.extend(arc(Round { center: Point2::new(0.0, 0.0), radius: r, from: PI / 2.0, sweep: PI }, half));
    p
}

/// An L of 40 x 30, its arms 10 wide, the inner corner rounded by 4.
fn l_shape(q: usize) -> Vec<Point2> {
    let mut p = Vec::new();
    p.extend(side([Point2::new(0.0, 0.0), Point2::new(40.0, 0.0)], 4));
    p.extend(side([Point2::new(40.0, 0.0), Point2::new(40.0, 10.0)], 1));
    p.extend(side([Point2::new(40.0, 10.0), Point2::new(14.0, 10.0)], 3));
    // the inner corner turns the other way: about (14, 14) from straight down to straight left
    p.extend(arc(Round { center: Point2::new(14.0, 14.0), radius: 4.0, from: -PI / 2.0, sweep: -PI / 2.0 }, q));
    p.extend(side([Point2::new(10.0, 14.0), Point2::new(10.0, 30.0)], 2));
    p.extend(side([Point2::new(10.0, 30.0), Point2::new(0.0, 30.0)], 1));
    p.extend(side([Point2::new(0.0, 30.0), Point2::new(0.0, 0.0)], 3));
    p
}

/// Every point moved off its place by up to `amp` along a fixed pseudo-random direction: what an exported mesh does.
fn shaken(points: Vec<Point2>, amp: f64) -> Vec<Point2> {
    points
        .into_iter()
        .enumerate()
        .map(|(k, p)| {
            let a = (k as f64 * 2.399_963).sin() * PI;
            let s = ((k as f64 * 0.618_034).fract() * 2.0 - 1.0) * amp;
            Point2::new(p.x + s * a.cos(), p.y + s * a.sin())
        })
        .collect()
}

fn cases() -> Vec<Case> {
    let mut v = Vec::new();
    for q in [3, 6, 16] {
        v.push(Case {
            name: format!("rounded rectangle 40 x 30 R3, {q} chords a quarter"),
            points: rounded(Size { w: 40.0, h: 30.0 }, 3.0, q),
            lines: 4,
            arcs: 4,
            circles: 0,
            radii: vec![3.0; 4],
            smooth: 8,
        });
    }
    v.push(Case {
        name: "rectangle 40 x 30".into(),
        points: rounded(Size { w: 40.0, h: 30.0 }, 0.0, 1).into_iter().fold(Vec::<Point2>::new(), |mut a, p| {
            if a.last().is_none_or(|l| l.dist(p) > 1e-9) {
                a.push(p);
            }
            a
        }),
        lines: 4,
        arcs: 0,
        circles: 0,
        radii: vec![],
        smooth: 0,
    });
    for n in [12, 24, 64] {
        v.push(Case {
            name: format!("circle 10 across, {n} chords"),
            points: arc(Round { center: Point2::new(20.0, 15.0), radius: 5.0, from: 0.3, sweep: 2.0 * PI }, n),
            lines: 0,
            arcs: 0,
            circles: 1,
            radii: vec![5.0],
            smooth: 0,
        });
    }
    for half in [6, 12] {
        v.push(Case { name: format!("slot 20 long R4, {half} chords a half"), points: slot(Slot { length: 20.0, radius: 4.0 }, half), lines: 2, arcs: 2, circles: 0, radii: vec![4.0; 2], smooth: 4 });
    }
    for q in [4, 8] {
        v.push(Case { name: format!("L with the inner corner R4, {q} chords"), points: l_shape(q), lines: 6, arcs: 1, circles: 0, radii: vec![4.0], smooth: 2 });
    }
    v
}

fn check(c: &Case, fit: &Fit, failures: &mut Vec<String>) {
    let found = fit_loop(&c.points, fit);
    let count = |want: fn(&Piece) -> bool| found.pieces.iter().filter(|p| want(p)).count();
    let lines = count(|p| matches!(p, Piece::Line { .. }));
    let arcs = count(|p| matches!(p, Piece::Arc { .. }));
    let circles = count(|p| matches!(p, Piece::Circle { .. }));
    let splines = count(|p| matches!(p, Piece::Spline(_)));
    if (lines, arcs, circles, splines) != (c.lines, c.arcs, c.circles, 0) {
        failures.push(format!("{} (tol {}): {lines} lines, {arcs} arcs, {circles} circles, {splines} splines, not {} / {} / {}", c.name, fit.tol, c.lines, c.arcs, c.circles));
        return;
    }
    let mut radii: Vec<f64> = found
        .pieces
        .iter()
        .filter_map(|p| match p {
            Piece::Arc { radius, .. } | Piece::Circle { radius, .. } => Some(*radius),
            _ => None,
        })
        .collect();
    radii.sort_by(f64::total_cmp);
    for (got, want) in radii.iter().zip(&c.radii) {
        if (got - want).abs() > fit.tol {
            failures.push(format!("{}: a radius {got:.4}, not {want} within {}", c.name, fit.tol));
        }
    }
    let smooth = found.joins.iter().filter(|j| **j == Join::Smooth).count();
    if smooth != c.smooth {
        failures.push(format!("{}: {smooth} smooth joins, not {}", c.name, c.smooth));
    }
    // the pieces close the loop: each ends where the next begins
    for (k, p) in found.pieces.iter().enumerate() {
        let next = &found.pieces[(k + 1) % found.pieces.len()];
        if let (Some(e), Some(s)) = (p.end(), next.start()) {
            if e.dist(s) > 1e-9 {
                failures.push(format!("{}: piece {k} ends at {e:?}, the next begins at {s:?}", c.name));
            }
        }
    }
}

#[test]
fn every_shape_is_found_as_its_lines_arcs_and_circles() {
    let mut failures = Vec::new();
    for c in cases() {
        check(&c, &Fit::for_loop(&c.points), &mut failures);
        let noisy = Case { points: shaken(c.points.clone(), 0.02), name: format!("{}, shaken by 0.02", c.name), radii: c.radii.clone(), ..c };
        check(&noisy, &Fit { tol: NOISY_TOL, ..Fit::for_loop(&noisy.points) }, &mut failures);
    }
    assert!(failures.is_empty(), "{} of the shapes were found wrong:\n{}", failures.len(), failures.join("\n"));
}

/// A FREE CURVE IS A SPLINE: an ellipse of 20 x 10 (a cylinder cut at a slant) is no line and no arc within the
/// tolerance, and comes out as one spline through its points, not as dozens of lines.
#[test]
fn a_curve_that_is_no_arc_is_one_spline() {
    let points: Vec<Point2> = (0..48).map(|k| 2.0 * PI * k as f64 / 48.0).map(|a| Point2::new(10.0 * a.cos(), 5.0 * a.sin())).collect();
    let found = fit_loop(&points, &Fit::for_loop(&points));
    assert!(
        found.pieces.iter().all(|p| matches!(p, Piece::Spline(_))) && !found.pieces.is_empty() && found.pieces.len() <= 2,
        "the ellipse is found as {:?}",
        found.pieces.iter().map(Piece::kind).collect::<Vec<_>>()
    );
}
