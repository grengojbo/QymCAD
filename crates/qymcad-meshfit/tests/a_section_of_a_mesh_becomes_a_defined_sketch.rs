//! A SECTION OF A MESH BECOMES A SKETCH DEFINED TO THE END: a plate of 40 x 30, its corners rounded R3, a hole of 10
//! through it, built exact, made a mesh as a file of a printer holds it, cut at half its height - and the sketch laid
//! from the section holds the plate's own sizes, rounded to a tenth, and no freedom.
use qymcad_core::feature::{BasePlane, PlaneFrame, SketchPlane};
use qymcad_core::geom::{encode_loops, mesh_section_loops, Point2, ProfEdge};
use qymcad_core::model::{FromLoops, Project};
use qymcad_meshfit::section::{fit_loop, Fit, Piece};
use std::f64::consts::FRAC_PI_2;

/// The plate's outline: a rectangle of 40 x 30 at (5, 5), its corners rounded by 3; and its hole, 10 across at (25, 20).
fn plate() -> Vec<f64> {
    let (x0, y0, x1, y1, r) = (5.0, 5.0, 45.0, 35.0, 3.0);
    let p = Point2::new;
    let round =
        |c: Point2, from: f64| ProfEdge::Arc { a: p(c.x + r * from.cos(), c.y + r * from.sin()), b: p(c.x + r * (from + FRAC_PI_2).cos(), c.y + r * (from + FRAC_PI_2).sin()), center: c, ccw: true };
    let outer = [
        ProfEdge::Line { a: p(x0 + r, y0), b: p(x1 - r, y0) },
        round(p(x1 - r, y0 + r), -FRAC_PI_2),
        ProfEdge::Line { a: p(x1, y0 + r), b: p(x1, y1 - r) },
        round(p(x1 - r, y1 - r), 0.0),
        ProfEdge::Line { a: p(x1 - r, y1), b: p(x0 + r, y1) },
        round(p(x0 + r, y1 - r), FRAC_PI_2),
        ProfEdge::Line { a: p(x0, y1 - r), b: p(x0, y0 + r) },
        round(p(x0 + r, y0 + r), 2.0 * FRAC_PI_2),
    ];
    let c = p(25.0, 20.0);
    let hole = [ProfEdge::Arc { a: p(30.0, 20.0), b: p(20.0, 20.0), center: c, ccw: false }, ProfEdge::Arc { a: p(20.0, 20.0), b: p(30.0, 20.0), center: c, ccw: false }];
    encode_loops(&[&outer, &hole])
}

/// How a mesh is shaken: by nothing, or by up to `amp` across, each corner its own way - what a file written to a few
/// hundredths does.
#[derive(Clone, Copy)]
struct Shake {
    amp: f64,
}

/// The plate as a mesh, cut at half its height, fitted, and laid as a sketch: the failures it shows.
fn drawn(shake: Shake, tol: Option<f64>) -> Vec<String> {
    let body = qymcad_kernel::specimen::extrude_profile(&plate(), 10.0).expect("the plate is built");
    let qymcad_core::geom::Built { mut mesh, .. } = qymcad_kernel::specimen::mesh(&body, 0.05).into_iter().next().expect("the plate is a mesh");
    // each corner shaken by where it stands, not by its number: a mesh repeats a corner for every face it bounds, and the
    // copies must move as one or the mesh is torn
    for v in mesh.verts.iter_mut() {
        let k = (v.x * 1000.0).round() * 7.0 + (v.y * 1000.0).round() * 13.0 + (v.z * 1000.0).round() * 17.0;
        let a = (k * 2.399_963).sin() * std::f64::consts::PI;
        let s = ((k * 0.618_034).fract().abs() * 2.0 - 1.0) * shake.amp;
        v.x += s * a.cos();
        v.y += s * a.sin();
    }
    let mut failures = Vec::new();
    let frame = PlaneFrame { origin: [0.0, 0.0, 5.0], x: [1.0, 0.0, 0.0], y: [0.0, 1.0, 0.0] };
    let loops = mesh_section_loops(&mesh, &frame);
    assert_eq!(loops.len(), 2, "the outline and the hole");
    let fitted: Vec<_> = loops.iter().map(|l| fit_loop(&l.points, &Fit { tol: tol.unwrap_or(Fit::for_loop(&l.points).tol), ..Fit::for_loop(&l.points) })).collect();
    let count = |want: fn(&Piece) -> bool| fitted.iter().flat_map(|f| &f.pieces).filter(|p| want(p)).count();
    let found = [
        count(|p| matches!(p, Piece::Line { .. })),
        count(|p| matches!(p, Piece::Arc { .. })),
        count(|p| matches!(p, Piece::Circle { .. })),
        count(|p| matches!(p, Piece::Spline(_))),
    ];
    if found != [4, 4, 1, 0] {
        return vec![format!("the section is {found:?} lines, arcs, circles and splines, not [4, 4, 1, 0]")];
    }
    let mut p = Project::default();
    let made = p.add_sketch_of_loops(FromLoops { plane: SketchPlane::World(BasePlane::XY), loops: &fitted, name: "Section".into() }).expect("a sketch is laid");
    if made.defined.free != 0 {
        failures.push(format!("the sketch is left free: {:?}", made.defined));
    }
    let mut sizes: Vec<f64> = p.sketches[made.si].constraints.iter().filter_map(|c| c.dim_value()).collect();
    sizes.sort_by(f64::total_cmp);
    for want in [3.0, 10.0, 30.0, 40.0] {
        if !sizes.iter().any(|s| (s - want).abs() < 1e-9) {
            failures.push(format!("no size of {want} among {sizes:?}"));
        }
    }
    for s in &sizes {
        if ((s * 10.0).round() - s * 10.0).abs() > 1e-6 {
            failures.push(format!("a size {s} is not rounded to a tenth: {sizes:?}"));
        }
    }
    failures
}

#[test]
fn a_plate_cut_at_half_height_is_drawn_with_its_own_sizes() {
    let failures = drawn(Shake { amp: 0.0 }, None);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// SHAKEN BY 0.02 AND FITTED WITHIN 0.06, the sides stand a few hundredths of a degree off the axes and the arcs off
/// their tangents: the relations are taken as a mesh shows them, and the plate is drawn as it is.
#[test]
fn a_shaken_plate_is_drawn_with_its_own_sizes() {
    let failures = drawn(Shake { amp: 0.02 }, Some(0.06));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
