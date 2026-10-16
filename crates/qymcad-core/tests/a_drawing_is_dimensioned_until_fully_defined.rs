//! A DRAWING IS DIMENSIONED UNTIL IT IS FULLY DEFINED: loose curves - what a face copied into a sketch leaves - are given
//! the relations they show (horizontal, vertical, parallel, perpendicular, tangent, concentric, equal) and then sizes, one
//! at a time, each kept only when it takes a freedom away, until none is left. Nothing moves, nothing is redundant, and
//! the place of the drawing is measured from what it is anchored to: the axes of the sketch, or the lines given.
//!
//! Drawing x anchor: failures are gathered and told together.
use qymcad_core::feature::{Purpose, Winding};
use qymcad_core::geom::Point2;
use qymcad_core::model::{Anchor, Constraint, Id, Project};

/// One drawing: its name and how it is laid into sketch `si`, answering the ids of its curves.
struct Drawing {
    name: &'static str,
    lay: fn(&mut Project, usize) -> Vec<Id>,
}

fn last(p: &Project, si: usize) -> Id {
    p.sketches[si].entities.last().map(|e| e.id).expect("a curve")
}

fn polygon(p: &mut Project, si: usize, pts: &[[f64; 2]]) -> Vec<Id> {
    (0..pts.len())
        .map(|k| {
            let (a, b) = (pts[k], pts[(k + 1) % pts.len()]);
            p.add_line_entity(si, a[0], a[1], b[0], b[1], Purpose::Real)
        })
        .collect()
}

/// An arc counter-clockwise round `c` from `a` towards `b`.
struct Arc {
    c: [f64; 2],
    a: [f64; 2],
    b: [f64; 2],
}

fn arc(p: &mut Project, si: usize, r: Arc) -> Id {
    let Arc { c, a, b } = r;
    p.add_arc_entity(si, Point2::new(c[0], c[1]), Point2::new(a[0], a[1]), Point2::new(b[0], b[1]), Winding::Ccw, Purpose::Real);
    last(p, si)
}

/// A rectangle of 30 x 20 from (5, 7), its corners rounded by 3: four sides and four arcs.
fn rounded(p: &mut Project, si: usize) -> Vec<Id> {
    let (x0, y0, x1, y1, r) = (5.0, 7.0, 35.0, 27.0, 3.0);
    vec![
        p.add_line_entity(si, x0 + r, y0, x1 - r, y0, Purpose::Real),
        arc(p, si, Arc { c: [x1 - r, y0 + r], a: [x1 - r, y0], b: [x1, y0 + r] }),
        p.add_line_entity(si, x1, y0 + r, x1, y1 - r, Purpose::Real),
        arc(p, si, Arc { c: [x1 - r, y1 - r], a: [x1, y1 - r], b: [x1 - r, y1] }),
        p.add_line_entity(si, x1 - r, y1, x0 + r, y1, Purpose::Real),
        arc(p, si, Arc { c: [x0 + r, y1 - r], a: [x0 + r, y1], b: [x0, y1 - r] }),
        p.add_line_entity(si, x0, y1 - r, x0, y0 + r, Purpose::Real),
        arc(p, si, Arc { c: [x0 + r, y0 + r], a: [x0, y0 + r], b: [x0 + r, y0] }),
    ]
}

/// A slot: two half circles of 3 round (10, 10) and (30, 10) joined by two sides.
fn slot(p: &mut Project, si: usize) -> Vec<Id> {
    vec![
        p.add_line_entity(si, 10.0, 7.0, 30.0, 7.0, Purpose::Real),
        arc(p, si, Arc { c: [30.0, 10.0], a: [30.0, 7.0], b: [30.0, 13.0] }),
        p.add_line_entity(si, 30.0, 13.0, 10.0, 13.0, Purpose::Real),
        arc(p, si, Arc { c: [10.0, 10.0], a: [10.0, 13.0], b: [10.0, 7.0] }),
    ]
}

fn drawings() -> Vec<Drawing> {
    vec![
        Drawing { name: "rectangle", lay: |p, si| polygon(p, si, &[[5.0, 7.0], [35.0, 7.0], [35.0, 27.0], [5.0, 27.0]]) },
        Drawing { name: "L", lay: |p, si| polygon(p, si, &[[0.0, 0.0], [40.0, 0.0], [40.0, 10.0], [12.0, 10.0], [12.0, 30.0], [0.0, 30.0]]) },
        Drawing { name: "trapezoid", lay: |p, si| polygon(p, si, &[[2.0, 3.0], [42.0, 3.0], [27.0, 23.0], [17.0, 23.0]]) },
        Drawing {
            name: "rectangle turned 30 deg",
            lay: |p, si| {
                let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
                let at = |x: f64, y: f64| [4.0 + x * c - y * s, 6.0 + x * s + y * c];
                polygon(p, si, &[at(0.0, 0.0), at(30.0, 0.0), at(30.0, 20.0), at(0.0, 20.0)])
            },
        },
        Drawing { name: "rounded rectangle", lay: rounded },
        Drawing { name: "slot", lay: slot },
        Drawing { name: "circle", lay: |p, si| vec![p.add_circle_entity(si, 10.0, 12.0, 4.0, Purpose::Real)] },
        Drawing {
            name: "plate with a hole",
            lay: |p, si| {
                let mut ids = polygon(p, si, &[[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]]);
                ids.push(p.add_circle_entity(si, 15.0, 12.0, 5.0, Purpose::Real));
                ids
            },
        },
    ]
}

/// Where every point of sketch `si` stands, by its id.
fn places(p: &Project, si: usize) -> Vec<Point2> {
    p.sketches[si].points.iter().map(|q| Point2::new(q.x, q.y)).collect()
}

fn run(d: &Drawing) -> Result<(), String> {
    let mut p = Project::default();
    let si = p.new_sketch("S");
    let ents = (d.lay)(&mut p, si);
    let before = places(&p, si);
    let had = p.sketches[si].constraints.len();
    let done = p.fully_define(si, &ents, &Anchor::Origin);
    let (free, excess) = p.sketch_dof(si);
    if (free, excess) != (0, 0) {
        return Err(format!("{free} freedoms left, {excess} in excess; {done:?}"));
    }
    let moved = before.iter().zip(places(&p, si)).map(|(a, b)| (a.x - b.x).hypot(a.y - b.y)).fold(0.0, f64::max);
    if moved > 1e-6 {
        return Err(format!("a point moved {moved}"));
    }
    // the axes of the sketch are held where they belong when the place is measured from them: not a size of the drawing
    let system = p.sketches[si].system_ids();
    let added = p.sketches[si].constraints[had..].iter().filter(|c| !matches!(c, Constraint::Fixed { p: q } if system.contains(q))).count();
    if added != done.relations + done.sizes {
        return Err(format!("{added} constraints went on, the answer counts {} relations and {} sizes", done.relations, done.sizes));
    }
    // DEFINED IS HELD: every point pulled 1 mm aside and let go, the drawing comes back where it was; a freedom left
    // keeps it where the pull put it. The rank alone is not taken for it: a distance from a side that may turn reads
    // the turn in, and two sizes saying one thing both counted. Under the pull itself a held drawing gives too - 0.3 mm
    // for a rectangle turned 30 deg - and comes back.
    let ids: Vec<Id> = p.sketches[si].points.iter().map(|q| q.id).collect();
    for id in ids {
        let mut pulled = p.clone();
        let Some(q) = pulled.sketches[si].points.iter().find(|q| q.id == id).map(|q| Point2::new(q.x, q.y)) else { continue };
        pulled.solve_sketch_drag(si, Some((id, q.x + 1.0, q.y + 1.0)));
        pulled.solve_sketch(si);
        let slid = places(&p, si).iter().zip(places(&pulled, si)).map(|(a, b)| (a.x - b.x).hypot(a.y - b.y)).fold(0.0, f64::max);
        if slid > 1e-4 {
            return Err(format!("point {id} pulled aside and let go, the drawing stays {slid} off"));
        }
    }
    if p.sketches[si].constraints.iter().any(|c| c.is_driven()) {
        return Err("a size went on as a reference".into());
    }
    Ok(())
}

#[test]
fn every_drawing_is_fully_defined_where_it_stands() {
    let mut wrong = Vec::new();
    for d in drawings() {
        if let Err(why) = run(&d) {
            wrong.push(format!("{}: {why}", d.name));
        }
    }
    assert!(wrong.is_empty(), "{} of {} went wrong:\n{}", wrong.len(), drawings().len(), wrong.join("\n"));
}

/// A rectangle copied off a crooked file - 29.97 x 20.02 from (5.03, 6.98) - is sized in round tenths: 30 x 20 from
/// (5, 7), and the answer says what was rounded.
#[test]
fn the_sizes_are_rounded_to_a_tenth() {
    let mut p = Project::default();
    let si = p.new_sketch("S");
    let ents = polygon(&mut p, si, &[[5.03, 6.98], [35.0, 6.98], [35.0, 27.0], [5.03, 27.0]]);
    let done = p.fully_define(si, &ents, &Anchor::Origin);
    assert_eq!(p.sketch_dof(si), (0, 0), "{done:?}");
    let xs: Vec<f64> = p.sketches[si].points.iter().filter(|q| !p.sketches[si].system_ids().contains(&q.id)).map(|q| q.x).collect();
    let ys: Vec<f64> = p.sketches[si].points.iter().filter(|q| !p.sketches[si].system_ids().contains(&q.id)).map(|q| q.y).collect();
    let span = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min);
    let low = |v: &[f64]| v.iter().cloned().fold(f64::MAX, f64::min);
    assert!((span(&xs) - 30.0).abs() < 1e-6 && (span(&ys) - 20.0).abs() < 1e-6, "the rectangle stands {} x {}, not 30 x 20", span(&xs), span(&ys));
    assert!((low(&xs) - 5.0).abs() < 1e-6 && (low(&ys) - 7.0).abs() < 1e-6, "the rectangle starts at ({}, {}), not (5, 7)", low(&xs), low(&ys));
    assert!(done.rounded.iter().any(|r| (r.was - 29.97).abs() < 1e-9 && (r.now - 30.0).abs() < 1e-9), "29.97 -> 30 is not told: {done:?}");
}

/// A boss of 20 x 10 drawn inside the outline of the plate it stands on, the outline held: the boss is placed by its
/// distances to the sides of the plate, not to the axes of the sketch.
#[test]
fn a_drawing_anchored_to_lines_is_placed_from_them() {
    let mut p = Project::default();
    let si = p.new_sketch("S");
    let plate = polygon(&mut p, si, &[[-5.0, -5.0], [55.0, -5.0], [55.0, 35.0], [-5.0, 35.0]]);
    let held: Vec<Id> = p.sketches[si].entities.iter().filter(|e| plate.contains(&e.id)).flat_map(qymcad_core::model::entity_points).collect::<std::collections::BTreeSet<Id>>().into_iter().collect();
    for q in held {
        p.sketches[si].constraints.push(Constraint::Fixed { p: q });
    }
    let boss = polygon(&mut p, si, &[[10.0, 8.0], [30.0, 8.0], [30.0, 18.0], [10.0, 18.0]]);
    let had = p.sketches[si].constraints.len();
    let done = p.fully_define(si, &boss, &Anchor::Lines(plate.clone()));
    assert_eq!(p.sketch_dof(si), (0, 0), "{done:?}");
    let system = p.sketches[si].system_ids();
    let to_axes = p.sketches[si].constraints[had..].iter().filter(|c| c.points().iter().any(|q| system.contains(q))).count();
    assert_eq!(to_axes, 0, "the boss is measured from the axes of the sketch, not from the plate");
}
