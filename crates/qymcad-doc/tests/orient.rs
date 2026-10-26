//! A BODY LAID ON A FACE: the move that turns the face asked to look straight down and sets the body's lowest point on
//! z = 0, read back on the body as it then stands - a box of 40 x 30 x 10 laid on its side of +X stands 40 tall, its
//! volume kept; a box in a part placed turned and lifted lands on the bed all the same.
use qymcad_core::model::Id;
use qymcad_doc::print::orient::lay_on;
use qymcad_doc::print::world_mesh;
use qymcad_doc::DocEngine;

/// A document with a box of 40 x 30 x 10 standing on XY about the origin.
struct Boxed {
    d: DocEngine,
    body: Id,
}

fn boxed() -> Boxed {
    let mut d = DocEngine::blank();
    let body = d
        .edit("box", |p| {
            let b = p.add_box(40.0, 30.0, 10.0);
            Ok(p.finish_base_body(b, 1))
        })
        .expect("the box is laid");
    Boxed { d, body }
}

/// The box of `body` in the world: its lowest and its highest corner.
fn span(d: &DocEngine, body: Id) -> [[f64; 3]; 2] {
    let m = world_mesh(d.project(), body).expect("the body stands");
    let b = m.bounds().expect("triangles");
    [[b.min.x, b.min.y, b.min.z], [b.max.x, b.max.y, b.max.z]]
}

/// Lays `body` on the face looking `down` as one move; the body the move makes.
fn laid(d: &mut DocEngine, body: Id, down: [f64; 3]) -> Id {
    let m = lay_on(d.project(), body, down).expect("a move");
    d.edit("lay", |p| Ok(p.add_move(body, m))).expect("the move is laid");
    d.project().timeline.last().and_then(|n| n.kind.body()).expect("the moved body")
}

#[test]
fn a_box_laid_on_its_side_stands_on_it() {
    let Boxed { mut d, body } = boxed();
    let before = d.shape(body).map(|s| s.volume()).expect("a live body");
    let moved = laid(&mut d, body, [1.0, 0.0, 0.0]);
    let [lo, hi] = span(&d, moved);
    assert!(lo[2].abs() < 1e-9, "the lowest point stands at z = {}", lo[2]);
    let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    assert!((size[2] - 40.0).abs() < 1e-6 && (size[0] - 10.0).abs() < 1e-6 && (size[1] - 30.0).abs() < 1e-6, "laid on its side of +X the box is {size:?}");
    let after = d.shape(moved).map(|s| s.volume()).expect("a live body");
    assert!((after - before).abs() < before * 1e-9, "the volume went from {before} to {after}");
}

#[test]
fn a_face_already_down_only_drops_to_the_bed() {
    let Boxed { mut d, body } = boxed();
    let moved = laid(&mut d, body, [0.0, 0.0, -1.0]);
    let [lo, hi] = span(&d, moved);
    assert!(lo[2].abs() < 1e-9 && (hi[2] - 10.0).abs() < 1e-6 && (hi[0] - lo[0] - 40.0).abs() < 1e-6, "{lo:?} {hi:?}");
    // the top turned down: the box upside down, the same size
    let moved = laid(&mut d, moved, [0.0, 0.0, 1.0]);
    let [lo, hi] = span(&d, moved);
    assert!(lo[2].abs() < 1e-9 && (hi[2] - 10.0).abs() < 1e-6, "{lo:?} {hi:?}");
}

#[test]
fn a_box_in_a_part_placed_turned_and_lifted_lands_on_the_bed() {
    let Boxed { mut d, body } = boxed();
    let part = d.project().body_owner(body).expect("the box's part");
    // the part turned 90 deg about X and lifted 50: the box's side of +Y now looks up
    d.edit("place", |p| {
        let mut m = qymcad_core::feature::rot12_axis([0.0; 3], [1.0, 0.0, 0.0], 90.0);
        m[11] += 50.0;
        p.set_component_transform(part, m);
        Ok(())
    })
    .expect("the part is placed");
    let moved = laid(&mut d, body, [1.0, 0.0, 0.0]);
    let [lo, hi] = span(&d, moved);
    assert!(lo[2].abs() < 1e-9, "the lowest point stands at z = {}", lo[2]);
    assert!((hi[2] - lo[2] - 40.0).abs() < 1e-6, "laid on the side looking +X in the world the box is {} tall", hi[2] - lo[2]);
}
