//! JOINTS ACROSS A CUT on the real kernel, read by volume: a bar cut in two has a round peg of 6 x 8 grown from the
//! piece behind the cut and a socket 0.15 wider all round and deeper in the piece in front - each piece changed by
//! exactly the volume of its tool; a dowel cuts a socket in each and stands a pin of its own; a hex peg is a hexagon;
//! every piece comes out one sound solid.
use qymcad_core::feature::{PegKind, PegShape, PegSpec, PlaneAt};
use qymcad_core::model::Id;
use qymcad_doc::print::checks::Bed;
use qymcad_doc::print::split::split_to_fit;
use qymcad_doc::DocEngine;
use std::f64::consts::PI;

/// A bar of 100 x 20 x 20 cut across its middle: the document, the piece behind the cut, the one in front, the cut.
struct Cut {
    d: DocEngine,
    behind: Id,
    front: Id,
    at: PlaneAt,
}

fn cut_bar() -> Cut {
    let mut d = DocEngine::blank();
    let bar = d
        .edit("bar", |p| {
            let b = p.add_box(100.0, 20.0, 20.0);
            Ok(p.finish_base_body(b, 1))
        })
        .expect("the bar is laid");
    let pieces = split_to_fit(&mut d, bar, &Bed { size: [60.0, 60.0, 60.0] }, 0.0).expect("the bar is cut");
    assert_eq!(pieces.len(), 2, "setup: two pieces");
    let p = d.project();
    let lo_x = |b: Id| p.mesh_index(b).and_then(|i| p.bodies[i].mesh.bounds()).map_or(0.0, |bb| bb.min.x);
    let (behind, front) = if lo_x(pieces[0]) < lo_x(pieces[1]) { (pieces[0], pieces[1]) } else { (pieces[1], pieces[0]) };
    let bb = p.mesh_index(behind).and_then(|i| p.bodies[i].mesh.bounds()).expect("the piece behind");
    let at = PlaneAt { origin: [bb.max.x, (bb.min.y + bb.max.y) / 2.0, (bb.min.z + bb.max.z) / 2.0], normal: [1.0, 0.0, 0.0] };
    Cut { d, behind, front, at }
}

fn volume(d: &DocEngine, b: Id) -> f64 {
    d.shape(b).map(|s| s.volume()).unwrap_or(f64::NAN)
}

/// Lays the joints `joint` at the middle of the cut; the bodies they make.
fn joined(c: &mut Cut, joint: PegSpec) -> Vec<Id> {
    let (pieces, at) = ([c.behind, c.front], c.at);
    let spot = at.origin;
    c.d.edit("joints", |p| Ok(p.add_joined(pieces, at, joint, vec![spot]))).expect("the joints are laid")
}

const ROUND: PegSpec = PegSpec { kind: PegKind::Plug, shape: PegShape::Round, diameter: 6.0, depth: 8.0, clearance: 0.15, taper: 0.0 };

#[test]
fn a_round_peg_grows_from_one_piece_into_a_socket_of_the_other() {
    let mut c = cut_bar();
    let (before_a, before_b) = (volume(&c.d, c.behind), volume(&c.d, c.front));
    let made = joined(&mut c, ROUND);
    assert_eq!(made.len(), 2, "{made:?}");
    let (a, b) = (volume(&c.d, made[0]), volume(&c.d, made[1]));
    let peg = PI * 3.0 * 3.0 * 8.0;
    let socket = PI * 3.15 * 3.15 * 8.15;
    assert!((a - before_a - peg).abs() < 1e-3, "the piece behind grew by {} against a peg of {peg}", a - before_a);
    assert!((before_b - b - socket).abs() < 1e-3, "the piece in front lost {} against a socket of {socket}", before_b - b);
    for &m in &made {
        let s = c.d.shape(m).expect("a live body");
        assert!(s.is_valid() && s.solid_count() == 1, "a piece is not one sound solid");
    }
}

#[test]
fn a_dowel_cuts_both_pieces_and_stands_a_pin_of_its_own() {
    let mut c = cut_bar();
    let (before_a, before_b) = (volume(&c.d, c.behind), volume(&c.d, c.front));
    let made = joined(&mut c, PegSpec { kind: PegKind::Dowel, ..ROUND });
    assert_eq!(made.len(), 3, "{made:?}");
    let socket = PI * 3.15 * 3.15 * 8.15;
    assert!((before_a - volume(&c.d, made[0]) - socket).abs() < 1e-3, "the piece behind lost no socket");
    assert!((before_b - volume(&c.d, made[1]) - socket).abs() < 1e-3, "the piece in front lost no socket");
    let pin = PI * 9.0 * 2.0 * (8.0 - 0.15);
    assert!((volume(&c.d, made[2]) - pin).abs() < 1e-3, "the pin holds {} against {pin}", volume(&c.d, made[2]));
}

#[test]
fn a_hex_peg_is_a_hexagon() {
    let mut c = cut_bar();
    let before_a = volume(&c.d, c.behind);
    let made = joined(&mut c, PegSpec { shape: PegShape::Hex, ..ROUND });
    // a hexagon on the circle of 3: 6 triangles of sides 3, area 3 sqrt(3) / 2 x 9, grown 8 past the cut
    let peg = 1.5 * 3f64.sqrt() * 9.0 * 8.0;
    assert!((volume(&c.d, made[0]) - before_a - peg).abs() < 1e-3, "the hex peg holds {}", volume(&c.d, made[0]) - before_a);
}

#[test]
fn a_tapered_peg_narrows_to_its_end() {
    let mut c = cut_bar();
    let before_a = volume(&c.d, c.behind);
    let made = joined(&mut c, PegSpec { taper: 5.0, ..ROUND });
    // a frustum from radius 3 at the cut to 3 - 8 tan 5 deg at its end
    let (r0, r1) = (3.0, 3.0 - 8.0 * 5f64.to_radians().tan());
    let peg = PI * 8.0 / 3.0 * (r0 * r0 + r0 * r1 + r1 * r1);
    assert!((volume(&c.d, made[0]) - before_a - peg).abs() < 1e-3, "the tapered peg holds {} against {peg}", volume(&c.d, made[0]) - before_a);
}

#[test]
fn the_sizes_are_the_nodes_numbers() {
    let mut c = cut_bar();
    let made = joined(&mut c, ROUND);
    let node = made[0];
    let kind = &c.d.project().timeline.iter().find(|n| n.id == node).expect("the node").kind;
    for key in ["diameter", "depth", "clearance", "taper"] {
        assert!(kind.dim(key).is_some(), "the node has no number {key}");
    }
}

/// THE SPOTS CHOSEN ON A CUT: a bar of 500 x 20 x 20 cut to fit a room of 256 has one cut; two round pegs stand on it
/// as far apart as the cut holds them, each its socket and the edge clear of the cut's rim; one hex peg holds alone; a
/// bar of 4 x 4 has no room for a peg of 6 and is told left without; all of it one step.
mod chosen {
    use super::*;
    use qymcad_doc::print::joints::{cut_and_join, JointAsk};

    /// A document and its bar.
    struct Bar {
        d: DocEngine,
        bar: Id,
    }

    fn bar(size: [f64; 3]) -> Bar {
        let mut d = DocEngine::blank();
        let bar = d
            .edit("bar", |p| {
                let b = p.add_box(size[0], size[1], size[2]);
                Ok(p.finish_base_body(b, 1))
            })
            .expect("the bar is laid");
        Bar { d, bar }
    }

    const ASK: JointAsk = JointAsk { spec: ROUND, edge: 0.8 };
    const ROOM: Bed = Bed { size: [256.0, 256.0, 256.0] };

    #[test]
    fn two_round_pegs_stand_apart_on_the_cut() {
        let Bar { mut d, bar } = bar([500.0, 20.0, 20.0]);
        let steps = d.history().undo_names().len();
        let done = cut_and_join(&mut d, bar, &ROOM, 5.0, &ASK).expect("cut and joined");
        assert_eq!(done.joints.len(), 1, "{done:?}");
        assert_eq!(done.left_without, 0, "{done:?}");
        let spots = &done.joints[0].spots;
        assert_eq!(spots.len(), 2, "{done:?}");
        // the cut is 20 x 20 about its middle: every socket of radius 3.15 and the edge of 0.8 within 10 of it
        let p = d.project();
        let bb = p.mesh_index(done.pieces[0]).and_then(|i| p.bodies[i].mesh.bounds()).expect("a piece");
        let (cy, cz) = ((bb.min.y + bb.max.y) / 2.0, (bb.min.z + bb.max.z) / 2.0);
        for s in spots {
            let off = (s[1] - cy).abs().max((s[2] - cz).abs());
            assert!(off + 3.15 + 0.8 <= 10.0 + 1e-6, "a spot {s:?} stands {off} off the middle of a cut of 20");
        }
        let apart = ((spots[0][1] - spots[1][1]).powi(2) + (spots[0][2] - spots[1][2]).powi(2)).sqrt();
        assert!(apart > 6.3, "two pegs of 6 stand {apart} apart: their sockets meet");
        assert_eq!(done.pieces.len(), 2, "{done:?}");
        assert_eq!(d.history().undo_names().len(), steps + 1, "the cut and its joints are not one step");
    }

    #[test]
    fn one_hex_peg_holds_alone() {
        let Bar { mut d, bar } = bar([500.0, 20.0, 20.0]);
        let done = cut_and_join(&mut d, bar, &ROOM, 5.0, &JointAsk { spec: PegSpec { shape: PegShape::Hex, ..ROUND }, edge: 0.8 }).expect("cut and joined");
        assert_eq!(done.joints[0].spots.len(), 1, "{done:?}");
    }

    #[test]
    fn a_cut_too_small_for_a_peg_is_told_left_without() {
        let Bar { mut d, bar } = bar([500.0, 4.0, 4.0]);
        let done = cut_and_join(&mut d, bar, &ROOM, 5.0, &ASK).expect("cut");
        assert!(done.joints.is_empty(), "{done:?}");
        assert_eq!(done.left_without, 1, "{done:?}");
        assert_eq!(done.pieces.len(), 2, "the bar is not cut: {done:?}");
    }
}

/// JOINTS SURVIVE THE FILE: the document saved and opened again holds the node and builds the pieces to the same
/// volumes.
#[test]
fn joints_are_kept_by_the_file() {
    let mut c = cut_bar();
    let made = joined(&mut c, ROUND);
    let want: Vec<f64> = made.iter().map(|&b| volume(&c.d, b)).collect();
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/qymcad-doc-joints"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let path = dir.join("joined.qcad").to_string_lossy().into_owned();
    c.d.save(&path, "2026-10-26T00:00:00Z").expect("saved");
    let mut back = DocEngine::open(&path).expect("opened");
    back.rebuild();
    assert!(back.project().timeline.iter().any(|n| matches!(n.kind, qymcad_core::feature::FeatureKind::Joined { .. })), "the joints are not in the file");
    let got: Vec<f64> = made.iter().map(|&b| volume(&back, b)).collect();
    for (g, w) in got.iter().zip(&want) {
        assert!((g - w).abs() < 1e-6, "a piece holds {g} opened again against {w}");
    }
}
