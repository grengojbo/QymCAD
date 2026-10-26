//! WHETHER A BODY PRINTS AS IT LIES, on bodies whose answer is known: a cube of 10 on the bed passes everything; a cube
//! of 300 does not fit a bed of 256, and fits turned no better; a box of 300 x 100 fits a bed of 256 x 320 only turned;
//! a cube lifted 5 mm is read where a slicer drops it; a cube on its edge touches the bed nowhere; a cube up on a post
//! hangs the 96 mm^2 of its bottom the post does not hold; a roof sloping 30
//! deg from the floor hangs over it past the 45 deg a printer bridges, one at 60 does not.
use qymcad_core::geom::{Mesh, Point3};
use qymcad_doc::print::checks::{contact, fits, overhang, Bed, Fit, Verdict};

/// A box: its sides, and the corner it stands from.
struct Block {
    size: [f64; 3],
    at: [f64; 3],
}

/// A box, its corners shared by its twelve triangles, wound outward.
fn cuboid(b: Block) -> Mesh {
    let Block { size, at } = b;
    let c = |i: usize| Point3::new(at[0] + (i & 1) as f64 * size[0], at[1] + ((i >> 1) & 1) as f64 * size[1], at[2] + ((i >> 2) & 1) as f64 * size[2]);
    let tris = vec![[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    Mesh { verts: (0..8).map(c).collect(), tris, ..Default::default() }
}

/// A wedge 10 long along y standing on a foot of 20 x 10: its bottom on the floor, a wall up at x = 0 of height 10, and
/// its underside a roof from the top of the wall down to x = 20 at `slope` deg from the floor - the face looking down.
/// Lifted on a post would hang; here it lies on its foot, the roof facing down over the bed from z = 10 at x = 0.
fn hanging_roof(slope: f64) -> Mesh {
    // a slab whose underside rises from the bed at x = 20 to a height at x = 0, standing on a strip at x = 20..21
    let h = 20.0 * slope.to_radians().tan();
    let p = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let verts = vec![
        p(0.0, 0.0, h),
        p(21.0, 0.0, 0.0),
        p(21.0, 0.0, h + 2.0),
        p(0.0, 0.0, h + 2.0),
        p(20.0, 0.0, 0.0),
        p(0.0, 10.0, h),
        p(21.0, 10.0, 0.0),
        p(21.0, 10.0, h + 2.0),
        p(0.0, 10.0, h + 2.0),
        p(20.0, 10.0, 0.0),
    ];
    // front (y = 0) and back (y = 10): the pentagon 0-4-1-2-3
    let mut tris = vec![[0, 4, 1], [0, 1, 2], [0, 2, 3], [5, 7, 6], [5, 8, 7], [5, 6, 9]];
    // the roof underside 0-4, the foot 4-1, the far end 1-2, the top 2-3, the near end 3-0
    for [a, b] in [[0, 4], [4, 1], [1, 2], [2, 3], [3, 0]] {
        tris.push([a, a + 5, b + 5]);
        tris.push([a, b + 5, b]);
    }
    Mesh { verts, tris, ..Default::default() }
}

const BED: Bed = Bed { size: [256.0, 256.0, 256.0] };

#[test]
fn a_cube_on_the_bed_prints() {
    let m = cuboid(Block { size: [10.0; 3], at: [0.0; 3] });
    assert_eq!(fits(&m, &BED).verdict, Verdict::Pass);
    let c = contact(&m);
    assert_eq!(c.verdict, Verdict::Pass, "{c:?}");
    assert!((c.area - 100.0).abs() < 1e-9, "{c:?}");
    let o = overhang(&m, 45.0);
    assert_eq!(o.verdict, Verdict::Pass, "{o:?}");
    assert!(o.area < 1e-9, "the bottom on the bed is counted as hanging: {o:?}");
}

#[test]
fn a_cube_past_the_bed_does_not_fit() {
    let f = fits(&cuboid(Block { size: [300.0; 3], at: [0.0; 3] }), &BED);
    assert_eq!(f.verdict, Verdict::Fail, "{f:?}");
    assert_eq!(f.fit, Fit::Not, "{f:?}");
    // a bed of 256 x 320: a box of 300 x 100 fits it turned a quarter about z, not as it lies
    let long = Bed { size: [256.0, 320.0, 256.0] };
    let f = fits(&cuboid(Block { size: [300.0, 100.0, 10.0], at: [0.0; 3] }), &long);
    assert_eq!(f.fit, Fit::Turned, "{f:?}");
    assert_eq!(f.verdict, Verdict::Warn, "{f:?}");
}

#[test]
fn a_cube_lifted_off_the_bed_is_read_where_a_slicer_drops_it() {
    // a slicer drops a part onto its bed: lifted 5 mm, the cube stands as it would on the floor
    let m = cuboid(Block { size: [10.0; 3], at: [0.0, 0.0, 5.0] });
    let c = contact(&m);
    assert_eq!(c.verdict, Verdict::Pass, "{c:?}");
    assert!((c.area - 100.0).abs() < 1e-9, "{c:?}");
    assert!(overhang(&m, 45.0).area < 1e-9, "the bottom of a cube lifted is counted as hanging");
}

#[test]
fn a_cube_on_its_edge_touches_the_bed_nowhere() {
    // turned 45 deg about x, the cube stands on one edge: a line, no area for the first layer to hold
    let mut m = cuboid(Block { size: [10.0; 3], at: [0.0; 3] });
    let (c, s) = (45f64.to_radians().cos(), 45f64.to_radians().sin());
    for v in &mut m.verts {
        let (y, z) = (v.y, v.z);
        v.y = y * c - z * s;
        v.z = y * s + z * c;
    }
    let t = contact(&m);
    assert_eq!(t.verdict, Verdict::Fail, "{t:?}");
    assert!(t.area < 1e-9, "{t:?}");
}

/// A cube of 10 up on a post of 2 x 2 x 5: its bottom of 100 mm^2, less the 4 the post holds, hangs over the air.
#[test]
fn a_cube_on_a_post_hangs_its_bottom() {
    let mut m = cuboid(Block { size: [10.0; 3], at: [0.0, 0.0, 5.0] });
    m.tris.drain(0..2); // the bottom goes; a ring round the post closes it below
    let shift = m.verts.len() as u32;
    let post = cuboid(Block { size: [2.0, 2.0, 5.0], at: [4.0, 4.0, 0.0] });
    m.verts.extend(post.verts);
    // the post without its top, and the bottom of the cube as four strips round the top of the post
    m.tris.extend(post.tris.iter().enumerate().filter(|(k, _)| !(2..4).contains(k)).map(|(_, t)| t.map(|v| v + shift)));
    let p = |x: f64, y: f64| qymcad_core::geom::Point3::new(x, y, 5.0);
    let ring = m.verts.len() as u32;
    m.verts.extend([p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0), p(4.0, 4.0), p(6.0, 4.0), p(6.0, 6.0), p(4.0, 6.0)]);
    for k in 0..4u32 {
        let (o, o2, i, i2) = (ring + k, ring + (k + 1) % 4, ring + 4 + k, ring + 4 + (k + 1) % 4);
        m.tris.push([o, i2, o2]);
        m.tris.push([o, i, i2]);
    }
    let o = overhang(&m, 45.0);
    assert_eq!(o.verdict, Verdict::Warn, "{o:?}");
    assert!((o.area - 96.0).abs() < 1e-9, "{o:?}");
    assert_eq!(o.places.len(), 1, "one place hangs: {o:?}");
    let at = o.places[0].at;
    assert!((at[0] - 5.0).abs() < 1e-9 && (at[1] - 5.0).abs() < 1e-9 && (at[2] - 5.0).abs() < 1e-9, "the place is not the middle of the bottom: {at:?}");
}

#[test]
fn a_roof_hangs_past_the_angle_and_not_short_of_it() {
    // 30 deg from the floor is 60 from the upright: past 45, it hangs; 60 from the floor is 30 from the upright: it holds
    let low = overhang(&hanging_roof(30.0), 45.0);
    assert_eq!(low.verdict, Verdict::Warn, "{low:?}");
    let roof = 20.0 / 30f64.to_radians().cos() * 10.0;
    assert!((low.area - roof).abs() < 1e-6, "the roof of {roof} mm^2 is not what hangs: {low:?}");
    let steep = overhang(&hanging_roof(60.0), 45.0);
    assert_eq!(steep.verdict, Verdict::Pass, "a roof 30 deg from the upright is counted as hanging: {steep:?}");
}
