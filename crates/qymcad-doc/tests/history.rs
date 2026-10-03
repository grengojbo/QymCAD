//! UNDO AND REDO WITHOUT A WINDOW: after every step of a chain, a step back gives the state before it and a step
//! forward gives it again, every body with its live shape; a parameter taken back takes its geometry back; an import
//! taken back and put again keeps its live body and its source; an aborted action leaves no trace.
use qymcad_core::feature::{FaceKey, SketchPlane, PLACE_IDENTITY};
use qymcad_core::model::{Id, Param, Project};
use qymcad_core::refs::{Axis, Query, Ref};
use qymcad_doc::{brep, history, regen};
use qymcad_kernel::{import_step, write_step, Shape};
use std::collections::HashMap;

/// A document driven as the window drives one: every change an action, every action rebuilt.
struct Doc {
    p: Project,
    shapes: HashMap<Id, Shape>,
    shelved: HashMap<Id, Shape>,
    shelved_sources: HashMap<Id, Vec<u8>>,
    seen: HashMap<String, f64>,
    history: history::History,
}

impl Doc {
    fn new() -> Self {
        let mut p = Project::default();
        p.new_document();
        Doc { p, shapes: HashMap::new(), shelved: HashMap::new(), shelved_sources: HashMap::new(), seen: HashMap::new(), history: history::History::new(40) }
    }

    fn rebuild(&mut self) {
        let done = regen::run(&mut self.p, &mut self.shapes, &mut self.seen, None);
        assert!(done.report.errors.is_empty(), "the document did not build: {:?}", done.report.errors);
    }

    /// One action: `f` changes the document, the rebuild follows, the step is committed.
    fn step<R>(&mut self, name: &str, f: impl FnOnce(&mut Project, &mut HashMap<Id, Shape>) -> R) -> R {
        self.history.begin(name, &self.p);
        let r = f(&mut self.p, &mut self.shapes);
        self.rebuild();
        self.history.commit();
        r
    }

    fn put(&mut self, snap: Project) {
        let shelf = history::Shelf { shapes: &mut self.shelved, sources: &mut self.shelved_sources };
        let _ = history::restore_snapshot(&mut self.p, &mut self.shapes, shelf, snap);
        self.rebuild();
    }

    fn undo(&mut self) {
        let (_, snap) = self.history.undo(&self.p).expect("a step to take back");
        self.put(snap);
    }

    fn redo(&mut self) {
        let (_, snap) = self.history.redo(&self.p).expect("a step to put again");
        self.put(snap);
    }

    /// Every body that stands on its own: its volume, its faces and their names.
    fn fingerprint(&self) -> Vec<(Id, i64, usize, Vec<u32>)> {
        let consumed = self.p.consumed_bodies();
        let mut out: Vec<(Id, i64, usize, Vec<u32>)> = self
            .p
            .bodies
            .iter()
            .filter(|b| !consumed.contains(&b.id))
            .map(|b| {
                // to the micro-cubic millimetre: two rebuilds of one recipe give one body
                let v = self.shapes.get(&b.id).map_or(b.mesh.volume(), |s| s.volume());
                let mut names: Vec<u32> = b.faces.iter().map(|f| f.id).collect();
                names.sort_unstable();
                (b.id, (v * 1e6).round() as i64, b.faces.len(), names)
            })
            .collect();
        out.sort();
        out
    }

    /// Every node that waits for a live body has one.
    fn every_body_is_live(&self) -> bool {
        brep::missing_brep(&self.p, &self.shapes).is_empty()
    }
}

/// A rectangle sketch on `plane`, closed, and its contour.
fn rect(p: &mut Project, name: &str, plane: Option<SketchPlane>, a: (f64, f64), b: (f64, f64)) -> (Id, Id) {
    let si = p.new_sketch(name);
    let sid = p.sketches[si].id;
    if let Some(pl) = plane {
        p.sketches[si].plane = pl;
    }
    p.add_sketch_node(sid, name);
    p.add_rect_entity(si, a.0, a.1, b.0, b.1, qymcad_core::feature::Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("a closed contour");
    (sid, cid)
}

fn block(p: &mut Project) -> Id {
    let (sid, cid) = rect(p, "base", None, (0.0, 0.0), (40.0, 30.0));
    let e = p.add_extrude_multi(sid, vec![cid], 10.0, qymcad_core::feature::Reach::Forward, 0.0, vec![]);
    p.finish_base_body(e, 1)
}

#[test]
fn every_step_of_a_chain_goes_back_and_comes_again() {
    let mut d = Doc::new();
    let mut states = vec![d.fingerprint()];
    let check = |d: &mut Doc, states: &mut Vec<_>, step: &str| {
        let now = d.fingerprint();
        assert!(d.every_body_is_live(), "{step}: a body has no live shape");
        d.undo();
        assert_eq!(d.fingerprint(), *states.last().expect("a state before"), "{step}: undo did not give the state before it");
        assert!(d.every_body_is_live(), "{step}: after undo a body has no live shape");
        d.redo();
        assert_eq!(d.fingerprint(), now, "{step}: redo did not give the step again");
        assert!(d.every_body_is_live(), "{step}: after redo a body has no live shape");
        states.push(now);
    };

    let blk = d.step("block", |p, _| block(p));
    check(&mut d, &mut states, "block");

    let top = d.p.regen_faces.get(&blk).and_then(|fs| fs.iter().find(|f| f.normal[2] > 0.9)).cloned().expect("the top face");
    let key = FaceKey { index: 0, centroid: [top.centroid.x, top.centroid.y, top.centroid.z], normal: top.normal, id: top.id };
    let cut = d.step("pocket", |p, _| {
        let (sid, cid) = rect(p, "pocket", Some(SketchPlane::Face(blk, key)), (10.0, 10.0), (30.0, 20.0));
        let span = qymcad_core::model::CombineSpan { height: 4.0, down: 0.0, extent: qymcad_core::feature::Extent { reach: qymcad_core::feature::Reach::Backward, ..Default::default() }, fill: &[] };
        p.add_combine_multi_op(blk, sid, vec![cid], span, 0)
    });
    check(&mut d, &mut states, "pocket");

    let fil = d.step("fillet", |p, _| p.add_fillet_ref(cut, 1.0, Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })))));
    check(&mut d, &mut states, "fillet");

    let _ = d.step("chamfer", |p, _| p.add_chamfer_ref(fil, 0.5, Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: false })))));
    check(&mut d, &mut states, "chamfer");
}

#[test]
fn a_parameter_taken_back_takes_its_geometry_back() {
    let mut d = Doc::new();
    // the block first and the chamfer on its built top face after, as a person picks the edges of a body standing
    let b = d.step("block", |p, _| {
        p.parameters.push(Param { name: "k".into(), expr: "1.5".into(), value: 1.5 });
        block(p)
    });
    let ch = d.step("chamfer by k", |p, _| {
        let c = p.add_chamfer_ref(b, 1.5, Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true }))));
        p.set_feat_dim(c, "dist", "k".into());
        c
    });
    let at_15 = d.shapes[&ch].volume();
    // 1.5^2 / 2 over the 140 mm of the top edges, less where the corners meet
    assert!((12000.0 - at_15 - 157.5).abs() < 6.0, "the chamfer at 1.5 took {} mm^3, not about 157.5", 12000.0 - at_15);
    d.step("k = 3", |p, _| {
        p.parameters[0].expr = "3".into();
        let _ = p.eval_parameters();
    });
    let at_3 = d.shapes[&ch].volume();
    assert!(at_3 < at_15 - 1.0, "a larger chamfer took no more material: {at_3} against {at_15}");
    d.undo();
    assert!((d.shapes[&ch].volume() - at_15).abs() < 1e-6, "k taken back to 1.5 left the chamfer at {} mm^3, not {at_15}", d.shapes[&ch].volume());
    d.redo();
    assert!((d.shapes[&ch].volume() - at_3).abs() < 1e-6, "k put again at 3 left the chamfer at {} mm^3, not {at_3}", d.shapes[&ch].volume());
}

#[test]
fn an_import_taken_back_and_put_again_keeps_its_live_body_and_its_source() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-history"));
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let path = dir.join("cube.step");
    let cube = Shape::extrude(&[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0], 10.0).expect("a cube");
    write_step(&[(&cube, PLACE_IDENTITY)], &path.to_string_lossy()).expect("the STEP is written");

    let mut d = Doc::new();
    let (body, source) = d.step("import", |p, shapes| {
        let source = p.add_source("cube.step", std::fs::read(&path).expect("the file reads"));
        let mesh = import_step(&path.to_string_lossy(), 0.5).expect("the STEP imports").remove(0).mesh;
        let body = p.add_mesh(mesh);
        p.import_tree_as_parts(vec![qymcad_core::model::ImportNode { name: "cube".into(), body: Some(body), ..Default::default() }], source, "cube");
        shapes.extend(brep::import_shapes(p));
        (body, source)
    });
    let bytes = |d: &Doc| d.p.sources.iter().find(|s| s.id == source).map_or(0, |s| s.data.len());
    let size = bytes(&d);
    assert!(size > 0 && d.shapes.contains_key(&body), "the import came in without its source or its live body");

    d.undo();
    assert!(d.p.mesh_index(body).is_none() && !d.shapes.contains_key(&body), "undo left the import in the document");
    assert!(d.shelved.contains_key(&body), "the live body of the import taken back was not kept for redo");

    d.redo();
    assert!(d.shapes.contains_key(&body), "the import came back from redo without its live body");
    assert_eq!(bytes(&d), size, "the import came back from redo without the bytes of its source");
    let named = d.p.regen_faces.get(&body).is_some_and(|fs| !fs.is_empty() && fs.iter().all(|f| f.id != 0));
    assert!(named, "the import came back from redo with its faces unnamed");
}

#[test]
fn an_aborted_action_leaves_no_trace_and_no_step() {
    let mut d = Doc::new();
    let _ = d.step("block", |p, _| block(p));
    let (before, ids_before) = (history::doc_key(&d.p), d.p.next_id);
    d.history.begin("a box", &d.p);
    let _ = d.p.add_box(5.0, 5.0, 5.0);
    let ids_handed = d.p.next_id;
    let snap = d.history.abort().expect("the state before the action");
    d.put(snap);
    // an id handed out once is never handed out again: the counter alone stays where the aborted action took it
    assert_eq!(d.p.next_id, ids_handed, "the ids the aborted action handed out would be handed out again");
    let mut same = d.p.clone();
    same.next_id = ids_before;
    assert_eq!(history::doc_key(&same), before, "the aborted action left the document changed");
    assert_eq!(d.history.undo_names(), vec!["block"], "the aborted action left a step behind");
}
