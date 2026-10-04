//! THE DOCUMENT DRIVEN THROUGH ONE DOOR: an action is one undo step with its rebuild; an action that refuses, or lays
//! a node on what is gone, leaves no trace and no step; a document opened from a file has every body live and comes
//! back the same after it is written and opened again.
use qymcad_core::feature::Reach;
use qymcad_core::model::{Id, Project};
use qymcad_core::refs::{Axis, Query, Ref};
use qymcad_doc::{history, DocEngine, DocError};

fn out_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-engine")).join(name);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    dir
}

/// A 40 x 30 x 10 block on the ground plane; the body it makes.
fn block(p: &mut Project) -> Id {
    let si = p.new_sketch("base");
    let sid = p.sketches[si].id;
    p.add_sketch_node(sid, "base");
    p.add_rect_entity(si, 0.0, 0.0, 40.0, 30.0, qymcad_core::feature::Purpose::Real);
    p.regen_sketch(si);
    let cid = p.sketches[si].contour_ids.iter().copied().find(|c| p.contour_profile_xy(*c).is_some()).expect("a closed contour");
    let e = p.add_extrude_multi(sid, vec![cid], 10.0, Reach::Forward, 0.0, vec![]);
    p.finish_base_body(e, 1)
}

/// The document without the id counter: an action taken back keeps the ids it handed out, by design.
fn key_of(p: &Project) -> u64 {
    let mut same = p.clone();
    same.next_id = 0;
    history::doc_key(&same)
}

/// Every body standing on its own, with its volume to the micro-cubic millimetre.
fn volumes(d: &DocEngine) -> Vec<(Id, i64)> {
    let p = d.project();
    let consumed = p.consumed_bodies();
    let mut out: Vec<(Id, i64)> = p.bodies.iter().filter(|b| !consumed.contains(&b.id)).map(|b| (b.id, (d.shape(b.id).map_or(b.mesh.volume(), |s| s.volume()) * 1e6).round() as i64)).collect();
    out.sort();
    out
}

#[test]
fn an_action_is_one_step_with_its_rebuild() {
    let mut d = DocEngine::blank();
    let body = d.edit("block", |p| Ok(block(p))).expect("the block is laid");
    let v = d.shape(body).map(|s| s.volume()).expect("the block has its live body after the action");
    assert!((v - 12000.0).abs() < 1e-6, "the block came out at {v} mm^3, not 12000");
    assert!(d.project().bodies.iter().any(|b| b.id == body && !b.faces.is_empty()), "the faces of the block did not go into the body");
    let fil = d.edit("fillet", |p| Ok(p.add_fillet_ref(body, 1.0, Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })))))).expect("the fillet is laid");
    assert!(d.report().errors.is_empty(), "the fillet did not build: {:?}", d.report().errors);
    assert_eq!(d.history().undo_names(), vec!["block", "fillet"], "each action is not one step");

    assert_eq!(d.undo().as_deref(), Some("fillet"));
    assert!(d.shape(fil).is_none(), "undo left the fillet's body live");
    assert_eq!(d.undo().as_deref(), Some("block"));
    assert!(d.shape(body).is_none() && d.project().mesh_index(body).is_none(), "undo left the block in the document");
    assert_eq!(d.undo(), None, "a step was taken back that no action made");
    assert_eq!(d.redo().as_deref(), Some("block"));
    assert_eq!(d.redo().as_deref(), Some("fillet"));
    assert!(d.shape(fil).is_some_and(|s| s.volume() < 12000.0), "redo did not put the fillet back live");
}

#[test]
fn a_refused_action_leaves_no_trace_and_no_step() {
    let mut d = DocEngine::blank();
    let _ = d.edit("block", |p| Ok(block(p))).expect("the block is laid");
    let before = (key_of(d.project()), volumes(&d));
    let r = d.edit("box", |p| {
        let _ = p.add_box(5.0, 5.0, 5.0);
        Err::<(), _>("refused-for-the-check".to_string())
    });
    assert_eq!(r, Err(DocError::Refused("refused-for-the-check".into())));
    assert_eq!((key_of(d.project()), volumes(&d)), before, "the refused action left the document changed");
    assert_eq!(d.history().undo_names(), vec!["block"], "the refused action left a step behind");
}

#[test]
fn a_node_laid_on_what_is_gone_is_not_laid() {
    let mut d = DocEngine::blank();
    let body = d.edit("block", |p| Ok(block(p))).expect("the block is laid");
    let before = (key_of(d.project()), volumes(&d));
    let gone: Id = 987_654_321;
    let r = d.edit("fillet", |p| Ok(p.add_fillet_ref(gone, 1.0, Ref::many(Query::Extreme { axis: Axis::Z, max: true }))));
    assert!(matches!(r, Err(DocError::Gone(ref n)) if n.len() == 1), "a fillet on a body that is not there was laid: {r:?}");
    assert_eq!((key_of(d.project()), volumes(&d)), before, "the action taken back left the document changed");
    assert!(d.shape(body).is_some(), "the action taken back took the live block with it");
    assert_eq!(d.history().undo_names(), vec!["block"], "the action taken back left a step behind");
}

#[test]
fn a_document_saved_and_opened_again_is_the_same() {
    let dir = out_dir("roundtrip");
    let path = dir.join("block.qcad").to_string_lossy().into_owned();
    let _ = std::fs::remove_file(&path);
    let mut d = DocEngine::blank();
    let body = d.edit("block", |p| Ok(block(p))).expect("the block is laid");
    let _ = d.edit("chamfer", |p| Ok(p.add_chamfer_ref(body, 1.0, Ref::many(Query::Adjacent(Box::new(Query::Extreme { axis: Axis::Z, max: true })))))).expect("the chamfer is laid");
    d.save(&path, "2026-10-04T12:00:00Z").expect("the document is written");
    d.save(&path, "2030-01-01T00:00:00Z").expect("the document is written again");
    assert_eq!(d.project().meta.created, "2026-10-04T12:00:00Z", "a second save moved the moment the document was started");

    let again = DocEngine::open(&path).expect("the document opens");
    assert!(again.report().errors.is_empty(), "the document opened red: {:?}", again.report().errors);
    assert!(again.report().built.is_empty(), "opening rebuilt {} bodies the file carries live", again.report().built.len());
    assert_eq!(volumes(&again), volumes(&d), "the document came back with other bodies");
    assert!(again.shape(body).is_some(), "the block came back from the file with no live body");
    assert!(again.history().undo_names().is_empty(), "opening a file made an undo step");
}

#[test]
fn a_file_that_is_not_there_is_an_error_not_a_document() {
    let r = DocEngine::open("/nonexistent/qymcad-doc/none.qcad");
    assert!(matches!(r, Err(DocError::File(ref code)) if code.starts_with("io-file-read#")), "opening nothing gave {:?}", r.err());
}

#[test]
fn an_empty_document_is_not_written_over_a_full_one() {
    let dir = out_dir("guard");
    let path = dir.join("full.qcad").to_string_lossy().into_owned();
    let _ = std::fs::remove_file(&path);
    let mut full = DocEngine::blank();
    let _ = full.edit("block", |p| Ok(block(p))).expect("the block is laid");
    full.save(&path, "2026-10-04T12:00:00Z").expect("the document is written");
    let r = DocEngine::new(Project::default()).save(&path, "2026-10-04T12:00:00Z");
    assert!(matches!(r, Err(DocError::File(ref code)) if code.starts_with("io-refuse-empty-over-full#")), "an empty document went over a full one: {r:?}");
}

/// The examples shipped with the program open with every body live, and come back the same after a write.
#[test]
fn the_examples_open_live_and_come_back_the_same() {
    let dir = out_dir("examples");
    for name in ["QymBoxPc-Case.qcad", "Filter-v2.qcad"] {
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/").to_string() + name;
        let mut d = DocEngine::open(&src).unwrap_or_else(|e| panic!("{name} does not open: {e:?}"));
        assert!(d.report().errors.is_empty(), "{name} opened red: {:?}", d.report().errors);
        d.ensure_brep();
        assert!(d.report().errors.is_empty(), "{name}: bringing the live bodies up went red: {:?}", d.report().errors);
        let waiting: Vec<Id> = d.project().timeline.iter().filter(|n| n.kind.waits_for_brep()).filter_map(|n| n.kind.body()).filter(|b| d.shape(*b).is_none()).collect();
        assert!(waiting.is_empty(), "{name}: bodies with no live shape: {waiting:?}");
        let out = dir.join(name).to_string_lossy().into_owned();
        let _ = std::fs::remove_file(&out);
        d.save(&out, "2026-10-04T12:00:00Z").expect("the example is written");
        let again = DocEngine::open(&out).unwrap_or_else(|e| panic!("{name} written does not open: {e:?}"));
        assert!(again.report().built.is_empty(), "{name}: opening rebuilt {} bodies the file carries live", again.report().built.len());
        let (a, b) = (volumes(&d), volumes(&again));
        assert_eq!(a.len(), b.len(), "{name} came back with another number of bodies");
        for ((ia, va), (ib, vb)) in a.iter().zip(&b) {
            assert_eq!(ia, ib, "{name} came back with other bodies");
            assert!((va - vb).abs() <= 1, "{name}: body {ia} came back at {vb} um^3, not {va}");
        }
    }
}
