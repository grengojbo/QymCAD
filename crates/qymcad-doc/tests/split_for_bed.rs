//! A BODY CUT TO FIT THE BED, on bodies whose cuts are known: a bar of 500 on a room of 256 with 5 to spare is cut once
//! across its middle into two of 250, their volumes the bar's, as one step undo takes back; a plate of 600 x 300 is cut
//! into 3 along its length and 2 across, six pieces; a body that fits is left whole and leaves no step.
use qymcad_core::model::Id;
use qymcad_doc::print::checks::Bed;
use qymcad_doc::print::split::{cuts_to_fit, split_to_fit};
use qymcad_doc::DocEngine;

const ROOM: Bed = Bed { size: [256.0, 256.0, 256.0] };

/// A document with a box of `size` standing on XY.
struct Boxed {
    d: DocEngine,
    body: Id,
}

fn boxed(size: [f64; 3]) -> Boxed {
    let mut d = DocEngine::blank();
    let body = d
        .edit("box", |p| {
            let b = p.add_box(size[0], size[1], size[2]);
            Ok(p.finish_base_body(b, 1))
        })
        .expect("the box is laid");
    Boxed { d, body }
}

/// The sides of the box of `body`'s mesh.
fn size_of(d: &DocEngine, body: Id) -> [f64; 3] {
    let p = d.project();
    let b = p.mesh_index(body).and_then(|i| p.bodies[i].mesh.bounds()).expect("the body stands");
    [b.max.x - b.min.x, b.max.y - b.min.y, b.max.z - b.min.z]
}

#[test]
fn the_cuts_of_a_bar_are_across_its_middle() {
    let cuts = cuts_to_fit([[0.0, 0.0, 0.0], [500.0, 20.0, 20.0]], &ROOM, 5.0);
    assert_eq!(cuts.len(), 1, "{cuts:?}");
    assert_eq!(cuts[0].axis, 0, "{cuts:?}");
    assert!((cuts[0].at - 250.0).abs() < 1e-9, "{cuts:?}");
    assert!(cuts_to_fit([[0.0; 3], [100.0, 100.0, 100.0]], &ROOM, 5.0).is_empty(), "a body that fits is cut");
}

#[test]
fn a_bar_past_the_bed_is_cut_in_two_as_one_step() {
    let Boxed { mut d, body } = boxed([500.0, 20.0, 20.0]);
    let whole = d.shape(body).map(|s| s.volume()).expect("a live body");
    let steps = d.history().undo_names().len();
    let pieces = split_to_fit(&mut d, body, &ROOM, 5.0).expect("the bar is cut");
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    for &p in &pieces {
        let s = size_of(&d, p);
        assert!(s[0] <= 251.0 + 1e-6, "a piece {s:?} does not fit the room less its margin");
    }
    let sum: f64 = pieces.iter().map(|&p| d.shape(p).map_or(0.0, |s| s.volume())).sum();
    assert!((sum - whole).abs() < whole * 1e-6, "the pieces hold {sum} against the bar's {whole}");
    assert_eq!(d.history().undo_names().len(), steps + 1, "the cut is not one step");
    d.undo();
    assert!(d.shape(body).is_some() && d.project().consumed_bodies().is_empty(), "undo did not bring the bar back whole");
}

#[test]
fn a_plate_past_the_bed_both_ways_is_cut_both_ways() {
    let Boxed { mut d, body } = boxed([600.0, 300.0, 20.0]);
    let pieces = split_to_fit(&mut d, body, &ROOM, 5.0).expect("the plate is cut");
    assert_eq!(pieces.len(), 6, "{pieces:?}");
    for &p in &pieces {
        let s = size_of(&d, p);
        assert!(s[0] <= 251.0 + 1e-6 && s[1] <= 251.0 + 1e-6, "a piece {s:?} does not fit");
    }
}

#[test]
fn a_body_that_fits_is_left_whole() {
    let Boxed { mut d, body } = boxed([100.0, 100.0, 100.0]);
    let steps = d.history().undo_names().len();
    assert_eq!(split_to_fit(&mut d, body, &ROOM, 5.0).expect("nothing to cut"), [body]);
    assert_eq!(d.history().undo_names().len(), steps, "a body that fits left a step");
}
