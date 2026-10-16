//! A size measured from a line whose direction is held only by Horizontal/Vertical relations still repeats a size
//! measured along the axis, and the count of freedoms must see it.
//!
//! Reported behaviour: an L of six lines, its sides held horizontal and vertical, its sizes laid as distances from
//! its bottom and right sides, then the width 40 laid again along X - the sketch read fully defined while the whole
//! drawing slid along X when a corner was dragged.
use qymcad_core::feature::Purpose;
use qymcad_core::model::{Constraint, Id, Project};

/// How the width 40 along X is laid.
enum Width {
    /// into the constraints as it is
    Pushed,
    /// through the door that keeps only what holds something
    Offered,
}

/// The sketch of the L and the ids its checks name.
struct L {
    p: Project,
    si: usize,
    corner: Id,
}

fn point_at(p: &Project, si: usize, (x, y): (f64, f64)) -> Id {
    p.sketches[si].points.iter().find(|q| (q.x - x).abs() < 1e-9 && (q.y - y).abs() < 1e-9).expect("a point of the L").id
}

fn span(p: &mut Project, si: usize, at: Id, line: [Id; 2], d: f64) {
    let [a, b] = line;
    p.sketches[si].constraints.push(Constraint::DistancePL { p: at, a, b, d, off: 0.0, expr: String::new(), driven: false, at: None });
}

/// Everything but the second PointOnLine: the L dimensioned from its bottom and right sides, the width 40 laid
/// again along X, the corner at zero on the X axis.
fn l_dimensioned(width: Width) -> L {
    let mut p = Project::default();
    p.new_document();
    let sid = p.add_sketch("L", vec![], None);
    p.add_sketch_node(sid, "Sketch");
    let si = p.sketch_index(sid).unwrap();
    let corners = [(0.0, 0.0), (40.0, 0.0), (40.0, 10.0), (12.0, 10.0), (12.0, 30.0), (0.0, 30.0)];
    for k in 0..corners.len() {
        let (a, b) = (corners[k], corners[(k + 1) % corners.len()]);
        p.add_line_entity(si, a.0, a.1, b.0, b.1, Purpose::Real);
    }
    let ids: Vec<Id> = corners.iter().map(|&(x, y)| point_at(&p, si, (x, y))).collect();
    for k in 0..ids.len() {
        let (a, b) = (ids[k], ids[(k + 1) % ids.len()]);
        let c = if k % 2 == 0 { Constraint::Horizontal { a, b } } else { Constraint::Vertical { a, b } };
        p.sketches[si].constraints.push(c);
    }
    let (frame, x_axis) = p.ensure_axis(si, 0);
    p.ensure_axis(si, 1);
    let bottom = [ids[0], ids[1]];
    let right = [ids[1], ids[2]];
    span(&mut p, si, ids[2], bottom, 10.0);
    span(&mut p, si, ids[4], bottom, 30.0);
    // the right side runs up, so a point to its left lies on its positive side
    span(&mut p, si, ids[3], right, 28.0);
    span(&mut p, si, ids[5], right, 40.0);
    let w = Constraint::Distance { a: ids[0], b: ids[1], d: 40.0, off: 0.0, expr: String::new(), driven: false, axis: 1, at: None };
    match width {
        Width::Pushed => p.sketches[si].constraints.push(w),
        Width::Offered => assert!(!p.add_constraint_if_independent(si, w), "the width 40 along X is refused: it holds nothing"),
    }
    p.sketches[si].constraints.push(Constraint::PointOnLine { p: ids[0], a: frame, b: x_axis });
    L { p, si, corner: ids[0] }
}

#[test]
fn the_width_laid_twice_is_redundant_and_the_l_slides_along_x() {
    let L { p, si, .. } = l_dimensioned(Width::Pushed);
    let width = p.sketches[si].constraints.iter().position(|c| matches!(c, Constraint::Distance { .. })).unwrap();
    assert!(p.dim_redundant(si, width), "the width 40 along X follows from the span 40 and the vertical sides");
    assert_eq!(p.sketch_dof(si), (1, 1), "one freedom left - along X - and one equation repeated");
    let s = &p.sketches[si];
    let on_x = s.constraints.iter().position(|c| matches!(c, Constraint::PointOnLine { .. })).unwrap();
    assert_eq!(qymcad_core::solver::freedom_taken(&s.points, &[], &s.constraints, on_x), 1, "the corner on the X axis takes the freedom along Y");
}

#[test]
fn the_width_offered_twice_is_refused_and_one_freedom_is_left() {
    let L { p, si, .. } = l_dimensioned(Width::Offered);
    assert_eq!(p.sketch_dof(si), (1, 0), "the L slides along X and nothing is repeated");
}

/// Whether a pull of `corner` by 1 mm, then a solve, leaves the drawing elsewhere than it stood.
fn slides(p: &mut Project, si: usize, corner: Id) -> bool {
    p.solve_sketch(si);
    let before: Vec<(f64, f64)> = p.sketches[si].points.iter().map(|q| (q.x, q.y)).collect();
    p.solve_sketch_drag(si, Some((corner, 1.0, 1.0)));
    p.solve_sketch(si);
    p.sketches[si].points.iter().zip(&before).any(|(q, b)| (q.x - b.0).abs() > 1e-4 || (q.y - b.1).abs() > 1e-4)
}

#[test]
fn a_sketch_read_fully_defined_comes_back_after_a_pull() {
    let L { mut p, si, corner } = l_dimensioned(Width::Pushed);
    let free = p.sketch_dof(si).0;
    assert_eq!(slides(&mut p, si, corner), free > 0, "the count says {free} free, the pull must agree");
    // the corner on the Y axis as well: now nothing is left, and the drawing comes back
    let (frame, y_axis) = p.ensure_axis(si, 1);
    p.sketches[si].constraints.push(Constraint::PointOnLine { p: corner, a: frame, b: y_axis });
    assert_eq!(p.sketch_dof(si).0, 0, "the corner on both axes leaves nothing free");
    assert!(!slides(&mut p, si, corner), "a sketch read fully defined comes back after a pull");
}
