//! A SECTION OF A MESH BECOMES A SKETCH DEFINED TO THE END: a plate of 40 x 30, its corners rounded R3, a hole of 10
//! through it, built exact, made a mesh as a file of a printer holds it, cut at half its height - and the sketch laid
//! from the section holds the plate's own sizes, rounded to a tenth, and no freedom.
use qymcad_core::feature::{BasePlane, PlaneFrame, SketchPlane};
use qymcad_core::geom::{encode_loops, mesh_section_loops, Point2, ProfEdge};
use qymcad_core::model::{FromLoops, Project};
use qymcad_meshfit::section::{fit_loop, Fit, Piece};
use std::f64::consts::FRAC_PI_2;

/// The plate's outline: a rectangle of 40 x 30 at (5, 5), its corners rounded by 3; and its hole, `bore` across at
/// (25, 20).
fn plate(bore: f64) -> Vec<f64> {
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
    let h = bore / 2.0;
    let hole = [ProfEdge::Arc { a: p(c.x + h, c.y), b: p(c.x - h, c.y), center: c, ccw: false }, ProfEdge::Arc { a: p(c.x - h, c.y), b: p(c.x + h, c.y), center: c, ccw: false }];
    encode_loops(&[&outer, &hole])
}

/// How a mesh is shaken: by nothing, or by up to `amp` across, each corner its own way - what a file written to a few
/// hundredths does.
#[derive(Clone, Copy)]
struct Shake {
    amp: f64,
}

/// The plate as a mesh, cut at half its height, fitted, and laid as a sketch: its sizes from the smallest, or the
/// failure it shows.
fn sizes_drawn(bore: f64, shake: Shake, tol: Option<f64>) -> Result<Vec<f64>, String> {
    let body = qymcad_kernel::specimen::extrude_profile(&plate(bore), 10.0).expect("the plate is built");
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
        return Err(format!("the section is {found:?} lines, arcs, circles and splines, not [4, 4, 1, 0]"));
    }
    let mut p = Project::default();
    let made = p.add_sketch_of_loops(FromLoops { plane: SketchPlane::World(BasePlane::XY), loops: &fitted, name: "Section".into() }).expect("a sketch is laid");
    if made.defined.free != 0 {
        return Err(format!("the sketch is left free: {:?}", made.defined));
    }
    let mut sizes: Vec<f64> = p.sketches[made.si].constraints.iter().filter_map(|c| c.dim_value()).collect();
    sizes.sort_by(f64::total_cmp);
    Ok(sizes)
}

/// The plate with its hole of 10 drawn: the failures it shows.
fn drawn(shake: Shake, tol: Option<f64>) -> Vec<String> {
    let sizes = match sizes_drawn(10.0, shake, tol) {
        Ok(s) => s,
        Err(e) => return vec![e],
    };
    let mut failures = Vec::new();
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

/// CUT 5 ABOVE THE XY PLANE, the sketch lies on a datum plane laid there, in the very frame the section was fitted in -
/// what the tool shows before Enter is what the sketch holds - and it is defined to the end.
#[test]
fn a_section_off_a_base_plane_lies_on_a_datum_plane_there() {
    use qymcad_core::model::{OfSection, SectionAt};
    use qymcad_meshfit::section::{fit_section, Within};
    let body = qymcad_kernel::specimen::extrude_profile(&plate(10.0), 10.0).expect("the plate is built");
    let qymcad_core::geom::Built { mesh, .. } = qymcad_kernel::specimen::mesh(&body, 0.05).into_iter().next().expect("the plate is a mesh");
    let mut p = Project::default();
    let id = p.alloc_id();
    p.bodies.push(qymcad_core::model::Body { id, mesh, ..Default::default() });
    let at = SectionAt { on: SketchPlane::World(BasePlane::XY), offset: 5.0 };
    let fit = |m: &qymcad_core::geom::Mesh, f: &PlaneFrame| fit_section(m, f, Within::OwnTolerance);
    let made = p.add_sketch_of_section(OfSection { body: id, at, fit: &fit, name: "Section".into() }).expect("a sketch is laid");
    assert!(matches!(p.sketches[made.si].plane, SketchPlane::Datum(_)), "the sketch lies on {:?}, not on a datum plane", p.sketches[made.si].plane);
    let (want, got) = (p.section_frame(&at).expect("the section has a frame"), p.sketch_frame(made.si).expect("the sketch has a frame"));
    let off = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| (a[k] - b[k]).abs()).fold(0.0, f64::max);
    assert!(off(want.origin, got.origin) < 1e-9 && off(want.x, got.x) < 1e-9 && off(want.y, got.y) < 1e-9, "the sketch's frame {got:?} is not the section's {want:?}");
    assert_eq!(made.defined.free, 0, "the sketch is left free: {:?}", made.defined);
    let missed = p.add_sketch_of_section(OfSection { body: id, at: SectionAt { on: SketchPlane::World(BasePlane::XY), offset: 50.0 }, fit: &fit, name: "Nothing".into() });
    assert!(missed.is_none() && p.sketches.len() == 1 && p.planes.len() == 1, "a plane missing the body laid something");
}

/// CUT OFF A DATUM PLANE OR OFF A FACE OF THE BODY ITSELF: a plane clicked, not only XY, XZ or YZ. A datum plane 2 above
/// XY moved by 3, and the top of the plate moved by -5, both cut the plate at half its height; each sketch lies on a
/// datum plane laid off what was clicked - off the plane, off the face - so the sketch moves when they move, in the
/// frame the section was fitted in, and holds the plate's own sizes with no freedom. The body begins and ends along the
/// face's normal 10 and 0 below it.
#[test]
fn a_section_off_a_datum_plane_or_a_face_lies_on_a_datum_plane_off_it() {
    use qymcad_core::model::{Extent, OfSection, PlaneDef, SectionAt, WorkPlane};
    use qymcad_meshfit::section::{fit_section, Within};
    let body = qymcad_kernel::specimen::extrude_profile(&plate(10.0), 10.0).expect("the plate is built");
    let qymcad_core::geom::Built { mesh, .. } = qymcad_kernel::specimen::mesh(&body, 0.05).into_iter().next().expect("the plate is a mesh");
    let mut p = Project::default();
    let id = p.alloc_id();
    let faces = qymcad_meshfit::raw_faces(&mesh);
    p.bodies.push(qymcad_core::model::Body { id, mesh, faces, ..Default::default() });
    let datum = p.add_plane(WorkPlane { origin: [0.0, 0.0, 2.0], normal: [0.0, 0.0, 1.0], ..Default::default() });
    let top = qymcad_core::feature::FaceKey { index: 0, centroid: [25.0, 20.0, 10.0], normal: [0.0, 0.0, 1.0], id: 0 };
    let extent = p.extent_along(id, &SketchPlane::Face(id, top)).map(|e| Extent { from: (e.from * 1e6).round() / 1e6, to: (e.to * 1e6).round() / 1e6 });
    assert_eq!(extent, Some(Extent { from: -10.0, to: 0.0 }), "the plate along the normal of its top");
    struct Case {
        name: &'static str,
        at: SectionAt,
        def: fn(&PlaneDef) -> bool,
    }
    let cases = [
        Case {
            name: "a datum plane 2 above XY, moved by 3",
            at: SectionAt { on: SketchPlane::Datum(datum), offset: 3.0 },
            def: |d| matches!(d, PlaneDef::OffsetPlane { dist, .. } if (*dist - 3.0).abs() < 1e-12),
        },
        Case {
            name: "the top of the plate, moved by -5",
            at: SectionAt { on: SketchPlane::Face(id, top), offset: -5.0 },
            def: |d| matches!(d, PlaneDef::OffsetFace { dist, .. } if (*dist + 5.0).abs() < 1e-12),
        },
    ];
    let fit = |m: &qymcad_core::geom::Mesh, f: &PlaneFrame| fit_section(m, f, Within::OwnTolerance);
    let off = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| (a[k] - b[k]).abs()).fold(0.0, f64::max);
    let mut failures = Vec::new();
    for c in cases {
        let Some(want) = p.section_frame(&c.at) else {
            failures.push(format!("{}: no frame", c.name));
            continue;
        };
        if (want.origin[2] - 5.0).abs() > 1e-9 {
            failures.push(format!("{}: the section is taken at {:?}, not at the height of 5", c.name, want.origin));
        }
        let Some(made) = p.add_sketch_of_section(OfSection { body: id, at: c.at, fit: &fit, name: c.name.into() }) else {
            failures.push(format!("{}: no sketch laid", c.name));
            continue;
        };
        let SketchPlane::Datum(laid) = p.sketches[made.si].plane else {
            failures.push(format!("{}: the sketch lies on {:?}, not on a datum plane", c.name, p.sketches[made.si].plane));
            continue;
        };
        let def = p.planes.iter().find(|w| w.id == laid).map(|w| w.def.clone());
        if !def.as_ref().is_some_and(c.def) {
            failures.push(format!("{}: the datum plane is laid as {def:?}", c.name));
        }
        let got = p.sketch_frame(made.si).expect("the sketch has a frame");
        if off(want.origin, got.origin) > 1e-9 || off(want.x, got.x) > 1e-9 || off(want.y, got.y) > 1e-9 {
            failures.push(format!("{}: the sketch's frame {got:?} is not the section's {want:?}", c.name));
        }
        if made.defined.free != 0 {
            failures.push(format!("{}: the sketch is left free: {:?}", c.name, made.defined));
        }
        let sizes: Vec<f64> = p.sketches[made.si].constraints.iter().filter_map(|k| k.dim_value()).collect();
        for want in [10.0, 30.0, 40.0] {
            if !sizes.iter().any(|s| (s - want).abs() < 1e-9) {
                failures.push(format!("{}: no size of {want} among {sizes:?}", c.name));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A FLAT FACE OF A MESH DRAWN BY ITS FITTED OUTLINE: the top of the plate - its outline chords, the rounded corners and
/// the hole a few dozen of them - is a sketch on the face of 4 lines, 4 arcs and a circle, defined to the end.
#[test]
fn the_top_of_a_mesh_plate_is_drawn_with_its_arcs_and_its_hole() {
    use qymcad_core::model::OfFace;
    let body = qymcad_kernel::specimen::extrude_profile(&plate(10.0), 10.0).expect("the plate is built");
    let qymcad_core::geom::Built { mesh, .. } = qymcad_kernel::specimen::mesh(&body, 0.05).into_iter().next().expect("the plate is a mesh");
    let mut p = Project::default();
    let id = p.alloc_id();
    let faces = qymcad_meshfit::raw_faces(&mesh);
    p.bodies.push(qymcad_core::model::Body { id, mesh, faces, ..Default::default() });
    let top = qymcad_core::feature::FaceKey { index: 0, centroid: [25.0, 20.0, 10.0], normal: [0.0, 0.0, 1.0], id: 0 };
    let fit = |pts: &[Point2]| fit_loop(pts, &Fit::for_loop(pts));
    let made = p.add_sketch_of_face_loops(OfFace { body: id, face: top, fit: &fit, name: "Top".into() }).expect("a sketch is laid");
    let s = &p.sketches[made.si];
    let kinds = |want: fn(&qymcad_core::model::EntityKind) -> bool| s.entities.iter().filter(|e| want(&e.kind)).count();
    let found = [kinds(|k| k.as_line().is_some()), kinds(|k| k.as_arc().is_some()), kinds(|k| k.as_circle().is_some())];
    assert_eq!(found, [4, 4, 1], "the top is drawn as lines, arcs and circles {found:?}");
    assert_eq!(made.defined.free, 0, "the sketch of the top is left free: {:?}", made.defined);
}

/// A PRINTED HOLE IS DRAWN AT ITS STANDARD BORE: a clearance hole for M5, drawn 5.5, comes out of a printer and its
/// mesh 5.42 across. A tenth makes it 5.4, which no screw is made for; the nearest bore within 0.15 is 5.5, and the
/// sketch says so. The same plate with a hole 7.73 across - no bore within 0.15 - keeps the tenth, 7.7.
#[test]
fn a_printed_hole_is_drawn_at_its_standard_bore() {
    struct Hole {
        measured: f64,
        laid: f64,
    }
    let mut failures = Vec::new();
    for h in [Hole { measured: 5.42, laid: 5.5 }, Hole { measured: 7.73, laid: 7.7 }] {
        match sizes_drawn(h.measured, Shake { amp: 0.0 }, None) {
            Ok(sizes) if sizes.iter().any(|s| (s - h.laid).abs() < 1e-9) => {}
            Ok(sizes) => failures.push(format!("a hole {} across is not laid {} among {sizes:?}", h.measured, h.laid)),
            Err(e) => failures.push(format!("a hole {} across: {e}", h.measured)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
