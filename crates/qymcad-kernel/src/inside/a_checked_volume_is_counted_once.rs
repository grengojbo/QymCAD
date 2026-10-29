//! THE VOLUME A CHECK ASKS FOR IS TO A PERCENT, THE ONE A PERSON READS IS EXACT, and neither is counted twice; the one
//! to a percent stays to a percent once the exact one is known, so that two bodies compared are read to one precision. Integrated
//! over spline faces the exact volume took 1.3-1.6 s a time on a propeller of 80 faces, and every node built on it counted
//! it three times - its source again, its result twice - for checks asking only whether a result holds any volume and fits
//! the box it came from. Reported behaviour: a chamfer of 34 edges took 7 s to rebuild, 4.2 s of it volumes.
use crate::Shape;

/// A block 40 x 30 x 10 with its long top front edge rounded R3.
fn a_rounded_block() -> Shape {
    let block = Shape::extrude(&[0.0, 0.0, 40.0, 0.0, 40.0, 30.0, 0.0, 30.0], 10.0).expect("the block");
    let edge = block.edges_info().into_iter().find(|e| e.circle.is_none() && e.poly.iter().all(|p| p[1].abs() < 1e-6 && (p[2] - 10.0).abs() < 1e-6)).expect("setup: the block has no top front edge");
    block.fillet_edges(3.0, &[edge.id]).expect("setup: R3 does not round an edge of a block 10 high")
}

#[test]
fn the_volume_read_is_exact_and_the_volume_checked_is_to_a_percent() {
    let rounded = a_rounded_block();
    // the corner square less the quarter disc, along the 40 of the edge
    let want = 12000.0 - (9.0 - std::f64::consts::PI * 9.0 / 4.0) * 40.0;
    let rough = rounded.rough_volume();
    assert!((rough - want).abs() <= 0.01 * want, "the volume checked is {rough}, more than a percent off {want}");
    let exact = rounded.volume();
    assert!((exact - want).abs() < 1e-6 * want, "the volume read is {exact}, not the exact {want}");
    assert_eq!(rounded.rough_volume(), rough, "the volume checked is another once the exact one is known: two bodies compared are read to one precision");
}

/// THE VOLUME CHECKED IS COUNTED ONCE TOO: asked again of the same body it answers from what it knows - a blend asked
/// it of its body and the door of its result, 20-50 ms a time on a body of 80 faces.
///
/// A call of microseconds is read on the wall, the least of five: a load only lengthens a call, so the least is the
/// call's own. Its time less its wait, read to a tick of the scheduler, came out 0 ns for the first call on a runner
/// under a load of 7.7 on 4 threads, and 9 runs of 60 went red beside 12 spinning threads.
#[test]
fn the_volume_checked_is_counted_once() {
    use qymcad_core::time_budget::{cost_of, Doer};
    const TIMES: usize = 5;
    // the body as it comes back from its bytes, as a trial build reads it: nothing known of it yet
    let bytes = a_rounded_block().to_brep_bytes().expect("the rounded block has no bytes");
    let fresh = || crate::specimen::read(&bytes).expect("the rounded block does not read back");
    let first = (0..TIMES).map(|_| fresh()).map(|rounded| cost_of(Doer::Thread, || rounded.rough_volume())).collect::<Vec<_>>();
    let rounded = fresh();
    let volume = rounded.rough_volume();
    let again = (0..TIMES).map(|_| cost_of(Doer::Thread, || rounded.rough_volume())).collect::<Vec<_>>();
    for t in first.iter().chain(&again) {
        assert_eq!(t.out, volume, "the volume checked again is another: {volume} then {}", t.out);
    }
    let least = |ts: &[qymcad_core::time_budget::Timed<f64>]| ts.iter().map(|t| t.cost.wall).min().unwrap_or_default();
    assert!(least(&again) * 10 <= least(&first), "the volume checked was counted again: {:?} the first time, {:?} again, the least of {TIMES} each", least(&first), least(&again));
}
