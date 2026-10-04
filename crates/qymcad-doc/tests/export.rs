//! A FILE WRITTEN OUT: only what is seen goes - not the base a cut ate, not a hidden part, only the subtree asked for;
//! an STL and a STEP read back hold the solid that was written; a 3MF keeps its parts by name and colour; a body with
//! no B-rep goes into a mesh file and is told missing from an exact one; writing makes no undo step.
use qymcad_core::feature::{Purpose, Reach, SketchPlane};
use qymcad_core::model::{Id, Project};
use qymcad_core::refs::{Axis, Query, Ref};
use qymcad_doc::export::{self, ExportTarget};
use qymcad_doc::import::MeshFormat;
use qymcad_doc::{DocEngine, DocError};

fn out_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-export")).join(name);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

fn same_name(s: &str) -> String {
    s.to_string()
}

/// A closed rectangle sketch on `plane` and its contour.
fn rect(p: &mut Project, plane: Option<SketchPlane>, a: (f64, f64), b: (f64, f64)) -> (Id, Id) {
    let si = p.new_sketch("s");
    let sid = p.sketches[si].id;
    if let Some(pl) = plane {
        p.sketches[si].plane = pl;
    }
    p.add_sketch_node(sid, "s");
    p.add_rect_entity(si, a.0, a.1, b.0, b.1, Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("a closed contour");
    (sid, cid)
}

/// A 40 x 30 x 10 block from `x0` in the active part; the body it makes.
fn block(p: &mut Project, x0: f64) -> Id {
    let (sid, cid) = rect(p, None, (x0, 0.0), (x0 + 40.0, 30.0));
    let e = p.add_extrude_multi(sid, vec![cid], 10.0, Reach::Forward, 0.0, vec![]);
    p.finish_base_body(e, 1)
}

/// A second part beside the first, made active; its id.
fn second_part(p: &mut Project, name: &str) -> Id {
    p.active_component = Some(p.root);
    let part = p.add_part(name);
    p.active_component = Some(part);
    part
}

fn volume(m: &qymcad_core::geom::Mesh) -> f64 {
    m.volume()
}

#[test]
fn only_what_is_seen_goes_out() {
    let mut d = DocEngine::blank();
    let base = d.edit("block", |p| Ok(block(p, 0.0))).expect("the block is laid");
    let top = d.project().regen_faces.get(&base).and_then(|fs| fs.iter().find(|f| f.normal[2] > 0.9)).cloned().expect("the top face");
    let key = qymcad_core::feature::FaceKey { index: 0, centroid: [top.centroid.x, top.centroid.y, top.centroid.z], normal: top.normal, id: top.id };
    let cut = d
        .edit("pocket", |p| {
            let (sid, cid) = rect(p, Some(SketchPlane::Face(base, key)), (10.0, 10.0), (30.0, 20.0));
            let span = qymcad_core::model::CombineSpan { height: 4.0, down: 0.0, extent: qymcad_core::feature::Extent { reach: Reach::Backward, ..Default::default() }, fill: &[] };
            Ok(p.add_combine_multi_op(base, sid, vec![cid], span, 0))
        })
        .expect("the pocket is cut");
    assert_eq!(export::visible_bodies(d.project(), ExportTarget::Project), vec![cut], "the base the cut ate went out with the cut");

    let (right, other) = d
        .edit("second part", |p| {
            let part = second_part(p, "Right");
            Ok((part, block(p, 100.0)))
        })
        .expect("the second part is laid");
    let mut all = export::visible_bodies(d.project(), ExportTarget::Project);
    all.sort_unstable();
    let mut want = vec![cut, other];
    want.sort_unstable();
    assert_eq!(all, want, "the project did not go out as its two parts");
    assert_eq!(export::visible_bodies(d.project(), ExportTarget::Component(right)), vec![other], "the part asked for went out with more than its own body");

    let _ = d.edit("hide", |p| Ok(p.set_component_visible(right, false))).expect("the part is hidden");
    assert_eq!(export::visible_bodies(d.project(), ExportTarget::Project), vec![cut], "a hidden part went out");
}

#[test]
fn an_stl_read_back_holds_the_solid_written() {
    let path = out_dir("stl").join("block.stl").to_string_lossy().into_owned();
    let mut d = DocEngine::blank();
    let body = d.edit("block", |p| Ok(block(p, 0.0))).expect("the block is laid");
    let _ = d.edit("fillet", |p| Ok(p.add_fillet_ref(body, 2.0, Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })))))).expect("the fillet is laid");
    let solid = d.project().bodies.iter().map(|b| b.id).find(|b| *b != body && d.shape(*b).is_some()).expect("the filleted body");
    let want = d.shape(solid).map(|s| s.volume()).expect("a live body");
    let out = d.export_mesh(&path, MeshFormat::Stl, ExportTarget::Project, 0.01, &same_name).expect("the STL is written");
    assert_eq!((out.written.bodies, out.written.failed), (1, 0), "the STL took {:?}", out.written);
    assert!(d.history().undo_names() == vec!["block", "fillet"], "writing a file made an undo step");

    let read = qymcad_io::import_stl_named(&path).expect("the STL reads back");
    let v: f64 = read.iter().map(|n| volume(&n.mesh)).sum();
    assert!((v - want).abs() < want * 2e-3, "the STL holds {v} mm^3 against the solid's {want}");
    let b = read[0].mesh.bounds().expect("the STL has extent");
    assert!((b.max.x - b.min.x - 40.0).abs() < 1e-6 && (b.max.z - b.min.z - 10.0).abs() < 1e-6, "the STL stands at {b:?}, not 40 x 30 x 10");
}

#[test]
fn a_step_read_back_holds_the_solid_and_its_part() {
    let path = out_dir("step").join("block.step").to_string_lossy().into_owned();
    let mut d = DocEngine::blank();
    let body = d.edit("block", |p| Ok(block(p, 0.0))).expect("the block is laid");
    let _ = d.edit("chamfer", |p| Ok(p.add_chamfer_ref(body, 1.0, Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })))))).expect("the chamfer is laid");
    let solid = d.project().bodies.iter().map(|b| b.id).find(|b| *b != body && d.shape(*b).is_some()).expect("the chamfered body");
    let want = d.shape(solid).map(|s| s.volume()).expect("a live body");
    let part = d.project().body_owner(solid).and_then(|o| d.project().components.iter().find(|c| c.id == o)).map(|c| c.name.clone()).expect("the part of the body");
    let out = d.export_exact(&path, qymcad_kernel::ExactFormat::Step, ExportTarget::Project, &same_name).expect("the STEP is written");
    assert_eq!(out.written.bodies, 1);

    let read = qymcad_kernel::read_exact_tree(qymcad_kernel::ExactFormat::Step, &path, 0.5).expect("the STEP reads back");
    assert_eq!(read.shapes.len(), 1, "the STEP holds {} solids, not the one written", read.shapes.len());
    let v = read.shapes[0].volume();
    assert!((v - want).abs() < want * 1e-6, "the STEP holds {v} mm^3 against the solid's {want}");
    assert!(read.nodes.iter().any(|n| n.name == part), "the part {part:?} is not in the STEP's tree: {:?}", read.nodes.iter().map(|n| &n.name).collect::<Vec<_>>());
}

#[test]
fn a_3mf_keeps_its_parts_by_name_and_colour() {
    let path = out_dir("3mf").join("two.3mf").to_string_lossy().into_owned();
    let mut d = DocEngine::blank();
    let left = d.edit("left", |p| Ok(block(p, 0.0))).expect("the left part is laid");
    let right = d
        .edit("right", |p| {
            let _ = second_part(p, "Right");
            Ok(block(p, 100.0))
        })
        .expect("the right part is laid");
    // a colour is given to a body that stands, as a person picks it on the screen
    d.edit("colours", |p| {
        for (b, c) in [(left, [200, 30, 30]), (right, [30, 30, 200])] {
            let i = p.mesh_index(b).ok_or("no body")?;
            p.set_mesh_color(i, c);
        }
        if let Some(c) = p.body_owner(left).and_then(|o| p.components.iter_mut().find(|c| c.id == o)) {
            c.name = "Left".into();
        }
        Ok(())
    })
    .expect("the colours are given");
    assert!(d.shape(left).is_some());
    let out = d.export_mesh(&path, MeshFormat::ThreeMf, ExportTarget::Project, 0.1, &same_name).expect("the 3MF is written");
    assert_eq!(out.written.bodies, 2);

    let read = qymcad_io::import_3mf(&path).expect("the 3MF reads back");
    let mut got: Vec<(String, Option<[u8; 3]>)> = read.iter().map(|n| (n.name.clone(), n.color)).collect();
    got.sort();
    assert_eq!(got, vec![("Left".to_string(), Some([200, 30, 30])), ("Right".to_string(), Some([30, 30, 200]))], "the 3MF came back with other parts");
}

#[test]
fn a_body_with_no_brep_goes_into_a_mesh_and_is_told_missing_from_a_step() {
    let dir = out_dir("mesh-only");
    let stl = dir.join("cube.stl").to_string_lossy().into_owned();
    let cube = qymcad_kernel::Shape::extrude(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0], 10.0).expect("a cube");
    let mesh = cube.tessellate_merged(0.1).expect("the cube tessellates").mesh;
    qymcad_io::export_stl(&[mesh], &stl).expect("the STL is written");
    let mut d = DocEngine::blank();
    let came = d.import(&stl, 1.0, &same_name).expect("the STL comes in");

    let r = d.export_exact(&dir.join("out.step").to_string_lossy(), qymcad_kernel::ExactFormat::Step, ExportTarget::Project, &same_name);
    assert_eq!(r, Err(DocError::File("io-exact-no-brep#STEP".into())), "a STEP of nothing exact was not refused as such");
    let out = d.export_mesh(&dir.join("out.stl").to_string_lossy(), MeshFormat::Stl, ExportTarget::Project, 0.1, &same_name).expect("the STL is written");
    assert_eq!(out.plan.mesh_only, came.bodies, "the mesh body was not told as going out as a mesh");
    assert_eq!(out.written.bodies, 1);

    let empty = DocEngine::blank().export_mesh(&dir.join("none.stl").to_string_lossy(), MeshFormat::Stl, ExportTarget::Project, 0.1, &same_name);
    assert_eq!(empty, Err(DocError::File("io-mesh-no-bodies#STL".into())), "an empty document wrote a file");
}
