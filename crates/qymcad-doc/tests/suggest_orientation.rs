//! WHICH FACE TO LAY A BODY ON, read on bodies whose best way is known: a cube up on a post hangs its bottom over the
//! air as it stands, and laid on its top it hangs nothing and stands on all 100 mm^2 of it - that comes first; a flat
//! plate comes first lying on its broad face, the lowest; nothing is changed by asking.
use qymcad_core::geom::{Mesh, Point3};
use qymcad_core::model::{Id, Project};
use qymcad_doc::print::orient::suggest;

/// A box: its sides and the corner it stands from.
struct Block {
    size: [f64; 3],
    at: [f64; 3],
}

fn cuboid(b: Block) -> Mesh {
    let Block { size, at } = b;
    let c = |i: usize| Point3::new(at[0] + (i & 1) as f64 * size[0], at[1] + ((i >> 1) & 1) as f64 * size[1], at[2] + ((i >> 2) & 1) as f64 * size[2]);
    let tris = vec![[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    Mesh { verts: (0..8).map(c).collect(), tris, ..Default::default() }
}

/// A cube of 10 up on a post of 2 x 2 x 5 standing on the floor.
fn cube_on_a_post() -> Mesh {
    let mut m = cuboid(Block { size: [10.0; 3], at: [0.0, 0.0, 5.0] });
    m.tris.drain(0..2);
    let shift = m.verts.len() as u32;
    let post = cuboid(Block { size: [2.0, 2.0, 5.0], at: [4.0, 4.0, 0.0] });
    m.verts.extend(post.verts);
    m.tris.extend(post.tris.iter().enumerate().filter(|(k, _)| !(2..4).contains(k)).map(|(_, t)| t.map(|v| v + shift)));
    let p = |x: f64, y: f64| Point3::new(x, y, 5.0);
    let ring = m.verts.len() as u32;
    m.verts.extend([p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0), p(4.0, 4.0), p(6.0, 4.0), p(6.0, 6.0), p(4.0, 6.0)]);
    for k in 0..4u32 {
        let (o, o2, i, i2) = (ring + k, ring + (k + 1) % 4, ring + 4 + k, ring + 4 + (k + 1) % 4);
        m.tris.push([o, i2, o2]);
        m.tris.push([o, i, i2]);
    }
    m
}

/// A document holding `mesh` as a body, its flat faces found as on a mesh brought in.
struct Held {
    p: Project,
    body: Id,
}

fn held(mesh: Mesh) -> Held {
    let mut p = Project::default();
    p.new_document();
    let faces = qymcad_meshfit::raw_faces(&mesh);
    let body = p.add_mesh(mesh);
    p.set_body_faces(body, faces);
    Held { p, body }
}

#[test]
fn a_cube_on_a_post_is_best_laid_on_its_top() {
    let Held { p, body } = held(cube_on_a_post());
    let before = p.bodies[0].mesh.clone();
    let ways = suggest(&p, body, 45.0);
    assert!(!ways.is_empty() && ways.len() <= 3, "{} ways", ways.len());
    let best = &ways[0];
    assert!(best.down[2] > 0.999, "the first way does not lay it on its top: {best:?}");
    assert!(best.overhang < 1e-6, "{best:?}");
    assert!((best.contact - 100.0).abs() < 1e-6, "{best:?}");
    // as it stands it hangs 96: no way told first may hang more than the one after it
    for w in ways.windows(2) {
        assert!(w[0].overhang <= w[1].overhang + 1e-6, "the ways are not from the least hanging: {ways:?}");
    }
    assert_eq!(p.bodies[0].mesh, before, "asking changed the body");
}

#[test]
fn a_plate_is_best_laid_flat() {
    let Held { p, body } = held(cuboid(Block { size: [60.0, 40.0, 3.0], at: [0.0, 0.0, 0.0] }));
    let best = &suggest(&p, body, 45.0)[0];
    assert!(best.down[2].abs() > 0.999, "the plate is not laid on a broad face: {best:?}");
    assert!((best.height - 3.0).abs() < 1e-6 && (best.contact - 2400.0).abs() < 1e-6, "{best:?}");
}
