//! A SECTION OF A MESH, DRAWN AND EXTRUDED, GIVES THE BODY BACK: a plate of 40 x 30, its corners rounded R3, a hole of 10
//! through it, built exact and made a mesh as a printer's file holds it; cut at half its height, the section drawn into a
//! sketch (`add_sketch_of_section`) and the sketch extruded as high as the plate - the body that comes out holds what
//! the plate held, within 0.5 %.
use qymcad_core::feature::{BasePlane, PlaneFrame};
use qymcad_core::geom::{encode_loops, Mesh, Point2, ProfEdge};
use qymcad_core::model::{OfSection, Project, SectionAt};
use qymcad_meshfit::section::{fit_section, Within};
use std::f64::consts::FRAC_PI_2;

/// The plate's outline at (5, 5), its corners rounded by 3, and its hole, 10 across at (25, 20).
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

#[test]
fn a_plate_drawn_by_its_section_and_extruded_holds_what_it_held() {
    let exact = qymcad_kernel::specimen::extrude_profile(&plate(), 10.0).expect("the plate is built");
    let want = exact.volume();
    let qymcad_core::geom::Built { mesh, .. } = qymcad_kernel::specimen::mesh(&exact, 0.05).into_iter().next().expect("the plate is a mesh");
    let mut p = Project::default();
    let c = p.add_part("A");
    p.set_active_component(Some(c));
    let mesh_body = p.alloc_id();
    p.bodies.push(qymcad_core::model::Body { id: mesh_body, mesh, ..Default::default() });
    let fit = |m: &Mesh, f: &PlaneFrame| fit_section(m, f, Within::OwnTolerance);
    let made = p.add_sketch_of_section(OfSection { body: mesh_body, at: SectionAt { base: BasePlane::XY, offset: 5.0 }, fit: &fit, name: "Section".into() }).expect("a sketch is laid");
    assert_eq!(made.defined.free, 0, "the sketch of the section is left free: {:?}", made.defined);
    // the outline taken as the profile, as a click inside the plate takes it - the hole inside it left open
    let outline = p.sketches[made.si].contour_ids.iter().copied().max_by(|a, b| {
        let area = |c: &qymcad_core::model::Id| p.contour_index(*c).map_or(0.0, |ci| p.contours[ci].area().abs());
        area(a).total_cmp(&area(b))
    });
    let body = p.add_extrude_multi(made.sketch, outline.into_iter().collect(), 10.0, qymcad_core::feature::Reach::Forward, 0.0, Vec::new());
    let (report, shapes) = qymcad_testkit::regenerate(&mut p);
    assert!(report.errors.is_empty(), "the extrusion did not build: {:?}", report.errors);
    let got = shapes.get(&body).map(|s| s.volume()).expect("the extrusion has a body");
    assert!((got - want).abs() <= 0.005 * want, "the plate drawn by its section holds {got:.1} mm^3, the plate held {want:.1}");
}
