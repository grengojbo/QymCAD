//! THE ROW OF A SIZE ON A BODY READS AS IT WAS TYPED: a gap of 15 between two faces is "Size on body 15", a hole moved 8
//! off a face "Hole position 8", as the rows of the features made exact read "Hole 8 through".
//!
//! Reported behaviour: "Size on body 15.0" and "Hole position 8.0" stood in the tree beside "Recognised hole 8 through".
use qymcad_core::feature::{FaceKey, GapMoves};
use qymcad_core::model::{Exact, Gap, HoleMove, Project, Reach, Remake};

/// The row of the node `node` as the tree writes it, its icon left out.
fn row(p: &Project, node: qymcad_core::model::Id) -> String {
    let ti = p.timeline.iter().position(|n| n.id == node).expect("the node in the timeline");
    crate::gui::panels_tree::feature_row_label(p, ti)
}

#[test]
fn a_whole_size_on_a_body_is_written_whole() {
    let mut p = Project::default();
    p.new_document();
    let block = p.add_box(40.0, 30.0, 10.0);
    let gap = p.add_face_gap(block, Gap { one: FaceKey::default(), other: FaceKey::default(), size: 15.0, moves: GapMoves::One });
    let hole = Remake { faces: Vec::new(), exact: Exact::Hole { entry: FaceKey::default(), at: [10.0, 15.0, 10.0], diameter: 10.0, depth: 10.0, reach: Reach::Through, rims: Vec::new() } };
    let moved = p.add_hole_moved(gap, HoleMove { hole, face: FaceKey::default(), size: 8.0 });
    let gap_row = row(&p, gap);
    assert!(gap_row.ends_with(&crate::i18n::tr1("feat-face-gap", "d", "15")), "the gap of 15 reads {gap_row:?}");
    let moved_row = row(&p, moved);
    assert!(moved_row.ends_with(&crate::i18n::tr1("feat-hole-moved", "d", "8")), "the hole moved 8 reads {moved_row:?}");
}
