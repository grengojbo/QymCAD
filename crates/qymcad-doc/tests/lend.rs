//! THE WINDOW'S DOCUMENT, LENT FOR ONE ACTION: taken as it stands - its live bodies are its own and are not built
//! again - changed through the same door, and handed back with every live body it holds, the untouched ones the very
//! ones that came in.
use qymcad_core::feature::Reach;
use qymcad_core::model::{Id, Project};
use qymcad_doc::{history, DocEngine, Lent};

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

/// A document with a block in it, handed out the way the window holds one.
fn window_with_block() -> (Lent, Id) {
    let mut d = DocEngine::blank();
    let body = d.edit("block", |p| Ok(block(p))).expect("the block is laid");
    (d.hand_back(), body)
}

/// THE LIVE BODIES ARE TAKEN AS THEY ARE. A body of the window is put under the block's id that the block's recipe
/// would never build - a 2 mm cube - and it is still there after lending, after an action on another body, and after
/// handing back: nothing was built again but what the action laid.
#[test]
fn a_lent_document_keeps_the_live_bodies_it_came_with() {
    let (mut lent, body) = window_with_block();
    let mut other = DocEngine::blank();
    let cube = other.edit("cube", |p| Ok(p.add_box(2.0, 2.0, 2.0))).expect("the cube is laid");
    let sentinel = other.hand_back().shapes.remove(&cube).expect("the cube is live");
    lent.shapes.insert(body, sentinel);

    let mut d = DocEngine::lend(lent);
    let v = d.shape(body).map(|s| s.volume()).expect("the lent body is live");
    assert!((v - 8.0).abs() < 1e-6, "lending built the block again: its body holds {v} mm^3, not the 8 it came with");
    let added = d.edit("box", |p| Ok(p.add_box(5.0, 5.0, 5.0))).expect("an action goes through on a lent document");

    let back = d.hand_back();
    let v = back.shapes.get(&body).map(|s| s.volume()).expect("the block's body came back");
    assert!((v - 8.0).abs() < 1e-6, "the action built an untouched body again: {v} mm^3, not 8");
    let v = back.shapes.get(&added).map(|s| s.volume()).expect("the action's body came back live");
    assert!((v - 125.0).abs() < 1e-6, "the action's body came back at {v} mm^3, not 125");
}

/// THE PARAMETERS COME LENT TOO. A block whose height reads the parameter `t` carries the same 2 mm cube in place of
/// its body: lent with the values the window last built with, an action on another body leaves it alone. Lent
/// without them, every reader of a parameter would count as changed and be built again.
#[test]
fn a_lent_document_does_not_build_what_reads_a_parameter_again() {
    let mut d = DocEngine::blank();
    let body = d
        .edit("block", |p| {
            p.parameters.push(qymcad_core::model::Param { name: "t".into(), expr: "10".into(), value: 10.0 });
            let b = block(p);
            let node = p.timeline.iter().rev().find(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::Extrude { .. })).map(|n| n.id).expect("the extrusion of the block");
            p.set_feat_dim(node, "height", "t".into());
            Ok(b)
        })
        .expect("the block is laid");
    let mut lent = d.hand_back();
    assert!(lent.params_seen.contains_key("t"), "the parameter the block was built with was not handed back");
    let mut other = DocEngine::blank();
    let cube = other.edit("cube", |p| Ok(p.add_box(2.0, 2.0, 2.0))).expect("the cube is laid");
    let sentinel = other.hand_back().shapes.remove(&cube).expect("the cube is live");
    lent.shapes.insert(body, sentinel);

    let mut d = DocEngine::lend(lent);
    let _ = d.edit("box", |p| Ok(p.add_box(5.0, 5.0, 5.0))).expect("an action goes through on a lent document");
    let v = d.shape(body).map(|s| s.volume()).expect("the block's body is live");
    assert!((v - 8.0).abs() < 1e-6, "an action built the reader of a parameter again: {v} mm^3, not the 8 it came with");
}

/// LENT AND HANDED BACK WITH NOTHING DONE, the document is the one that went out, and the history lent with it is
/// the history that comes back.
#[test]
fn a_document_handed_back_untouched_is_the_same() {
    let (lent, body) = window_with_block();
    let key = history::doc_key(&lent.project);
    let d = DocEngine::lend(lent);
    assert!(d.report().errors.is_empty() && d.history().undo_names().is_empty(), "lending made a step or a rebuild report");
    let back = d.hand_back();
    assert_eq!(history::doc_key(&back.project), key, "the document came back changed");
    assert!(back.shapes.contains_key(&body), "the block's live body did not come back");
}

/// AN ACTION ON A LENT DOCUMENT is the action on the same document opened from its file: the same bodies, the same
/// volumes.
#[test]
fn an_action_on_a_lent_document_is_the_action_on_an_opened_one() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-lend"));
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let path = dir.join(format!("block-{}.qcad", std::process::id()));
    let path = path.to_str().expect("a path in UTF-8");
    let mut first = DocEngine::blank();
    let body = first.edit("block", |p| Ok(block(p))).expect("the block is laid");
    first.save(path, "2026-10-07T00:00:00Z").expect("the block is written");

    let fillet = |p: &mut Project| {
        Ok(p.add_fillet_ref(body, 1.0, qymcad_core::refs::Ref::many(qymcad_core::refs::Query::Adjacent(Box::new(qymcad_core::refs::Query::Extreme { axis: qymcad_core::refs::Axis::Z, max: true })))))
    };
    let mut opened = DocEngine::open(path).expect("the block opens");
    let a = opened.edit("fillet", fillet).expect("the fillet goes through on the opened document");
    let mut lent = DocEngine::lend(first.hand_back());
    let b = lent.edit("fillet", fillet).expect("the fillet goes through on the lent document");
    let _ = std::fs::remove_file(path);

    let va = opened.shape(a).map(|s| s.volume()).expect("the fillet is live on the opened document");
    let vb = lent.shape(b).map(|s| s.volume()).expect("the fillet is live on the lent document");
    assert!((va - vb).abs() < 1e-6 * va, "the same fillet came out at {va} mm^3 opened and {vb} mm^3 lent");
    assert!(va < 12000.0, "the fillet took nothing off: {va} mm^3");
}
