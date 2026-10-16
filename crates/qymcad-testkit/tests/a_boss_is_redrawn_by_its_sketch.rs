//! A BOSS OR A POCKET IS REDRAWN BY A SKETCH TAKEN OFF ITS FACE, on a solid come from a STEP file: the sketch comes out
//! fully defined, placed from the outline of the plate; one node takes the boss off and lays it again by the sketch, the
//! body holding what it held; a size of the sketch typed anew, and the height in the tree, drive the body by what they
//! add. Built with the live kernel.
//!
//! Body x edit: failures are gathered and told together.
use qymcad_core::feature::FaceKey;
use qymcad_core::geom::MeshFace;
use qymcad_core::model::{Constraint, FromFace, Id, ImportNode, Project};
use qymcad_kernel::specimen::Specimen;
use qymcad_kernel::{OcctKernel, Shape};
use std::collections::HashMap;
use std::f64::consts::PI;

/// A box by its corner and its sizes.
struct Block {
    x: f64,
    y: f64,
    w: f64,
    d: f64,
    z: f64,
    h: f64,
}

/// A cylinder standing up from (x, y, z), of radius `r` and height `h`.
struct Post {
    x: f64,
    y: f64,
    r: f64,
    z: f64,
    h: f64,
}

fn block(b: Block) -> Shape {
    let Block { x, y, w, d, z, h } = b;
    let at = [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z];
    qymcad_kernel::specimen::extrude(&[0.0, 0.0, w, 0.0, w, d, 0.0, d], h).expect("a block").moved(&at).expect("placed")
}

fn post(p: Post) -> Shape {
    let Post { x, y, r, z, h } = p;
    let at = [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z];
    qymcad_kernel::specimen::cylinder(r, h).expect("a cylinder").moved(&at).expect("placed")
}

/// A plate of 60 x 40 x 5 with a boss of 30 x 20 x 8 from (10, 10) on it.
fn bossed() -> Shape {
    block(Block { x: 0.0, y: 0.0, w: 60.0, d: 40.0, z: 0.0, h: 5.0 }).combined(&block(Block { x: 10.0, y: 10.0, w: 30.0, d: 20.0, z: 5.0, h: 8.0 }), 1).expect("fused")
}

/// The edges at the corners of the boss of `bossed`, running up its walls.
fn uprights(s: &Shape) -> Vec<u32> {
    let corner = |p: [f32; 3]| [10.0, 40.0].iter().any(|x| (p[0] as f64 - x).abs() < 1e-6) && [10.0, 30.0].iter().any(|y| (p[1] as f64 - y).abs() < 1e-6);
    s.edges_info()
        .into_iter()
        .filter(|e| match (e.poly.first(), e.poly.last()) {
            (Some(a), Some(b)) => corner(*a) && corner(*b) && (a[2] - b[2]).abs() > 1.0,
            _ => false,
        })
        .map(|e| e.id)
        .collect()
}

/// The edges round the top of the boss of `bossed`, at z = 13.
fn top_rim(s: &Shape) -> Vec<u32> {
    s.edges_info().into_iter().filter(|e| e.poly.iter().all(|p| (p[2] as f64 - 13.0).abs() < 1e-6)).map(|e| e.id).collect()
}

/// The edges round the foot of the boss of `bossed`, at z = 5 inside the plate's outline.
fn foot(s: &Shape) -> Vec<u32> {
    s.edges_info().into_iter().filter(|e| e.poly.iter().all(|p| (p[2] as f64 - 5.0).abs() < 1e-6 && p[0] > 1.0 && p[0] < 59.0 && p[1] > 1.0 && p[1] < 39.0)).map(|e| e.id).collect()
}

/// A size of the sketch typed anew: from what it measures to what it is made.
struct Resize {
    from: f64,
    to: f64,
}

/// One case: the solid, the face clicked, the height typed anew in the tree and what the body then gains (less for a
/// pocket), and a size of the sketch typed anew after it and what that gains at the new height.
struct Case {
    name: &'static str,
    solid: fn() -> Shape,
    clicked: fn(&MeshFace) -> bool,
    resize: Resize,
    gains: f64,
    height: f64,
    height_gains: f64,
}

fn top(z: f64) -> impl Fn(&MeshFace) -> bool {
    move |f: &MeshFace| f.normal[2] > 0.999 && (f.centroid.z - z).abs() < 1e-6
}

fn cases() -> Vec<Case> {
    vec![
        Case { name: "boss", solid: bossed, clicked: |f| top(13.0)(f), resize: Resize { from: 30.0, to: 40.0 }, gains: 10.0 * 20.0 * 10.0, height: 10.0, height_gains: 30.0 * 20.0 * 2.0 },
        Case {
            name: "pocket",
            solid: || block(Block { x: 0.0, y: 0.0, w: 60.0, d: 40.0, z: 0.0, h: 10.0 }).combined(&block(Block { x: 10.0, y: 10.0, w: 30.0, d: 20.0, z: 6.0, h: 5.0 }), 0).expect("cut"),
            clicked: |f| top(6.0)(f),
            resize: Resize { from: 30.0, to: 40.0 },
            gains: -10.0 * 20.0 * 6.0,
            height: 6.0,
            height_gains: -30.0 * 20.0 * 2.0,
        },
        Case {
            name: "round post",
            solid: || block(Block { x: 0.0, y: 0.0, w: 60.0, d: 40.0, z: 0.0, h: 5.0 }).combined(&post(Post { x: 30.0, y: 20.0, r: 6.0, z: 5.0, h: 8.0 }), 1).expect("fused"),
            clicked: |f| top(13.0)(f),
            resize: Resize { from: 12.0, to: 16.0 },
            gains: PI * (64.0 - 36.0) * 10.0,
            height: 10.0,
            height_gains: PI * 36.0 * 2.0,
        },
        Case {
            name: "boss with its corners rounded",
            solid: || {
                let s = bossed();
                let e = uprights(&s);
                s.filleted(3.0, &e).expect("rounded")
            },
            clicked: |f| top(13.0)(f),
            resize: Resize { from: 30.0, to: 40.0 },
            gains: 10.0 * 20.0 * 10.0,
            height: 10.0,
            height_gains: (30.0 * 20.0 - (4.0 - PI) * 9.0) * 2.0,
        },
        // a rounding of 2 along the top: the boss 2 taller gains its section, the rounding moving up with the top; 10 longer,
        // its two long edges lose r^2 (1 - pi/4) a millimetre more each to the rounding laid again
        Case {
            name: "boss rounded at its top",
            solid: || {
                let s = bossed();
                let e = top_rim(&s);
                s.filleted(2.0, &e).expect("rounded")
            },
            clicked: |f| top(13.0)(f),
            resize: Resize { from: 30.0, to: 40.0 },
            gains: 10.0 * 20.0 * 10.0 - 2.0 * 10.0 * 4.0 * (1.0 - PI / 4.0),
            height: 10.0,
            height_gains: 30.0 * 20.0 * 2.0,
        },
        Case {
            name: "boss cut at its top",
            solid: || {
                let s = bossed();
                let e = top_rim(&s);
                s.chamfered(1.5, &e).expect("cut")
            },
            clicked: |f| top(13.0)(f),
            resize: Resize { from: 30.0, to: 40.0 },
            gains: 10.0 * 20.0 * 10.0 - 2.0 * 10.0 * 1.5 * 1.5 / 2.0,
            height: 10.0,
            height_gains: 30.0 * 20.0 * 2.0,
        },
        Case {
            name: "boss rounded at its foot",
            solid: || {
                let s = bossed();
                let e = foot(&s);
                s.filleted(1.0, &e).expect("rounded")
            },
            clicked: |f| top(13.0)(f),
            resize: Resize { from: 30.0, to: 40.0 },
            gains: 10.0 * 20.0 * 10.0 + 2.0 * 10.0 * (1.0 - PI / 4.0),
            height: 10.0,
            height_gains: 30.0 * 20.0 * 2.0,
        },
    ]
}

/// A document with a solid brought in: the body and the live shapes the rebuild left.
struct BroughtIn {
    p: Project,
    body: Id,
    shapes: HashMap<Id, Shape>,
}

/// The solid written to a STEP file and brought in as a person brings one.
fn brought_in(name: &str, s: &Shape) -> Result<BroughtIn, String> {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/prism-redrawn"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.step", name.replace(' ', "-")));
    qymcad_kernel::write_step(&[(s, qymcad_core::feature::PLACE_IDENTITY)], &path.to_string_lossy()).map_err(|e| format!("the STEP is not written: {e:?}"))?;
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("part.step", std::fs::read(&path).map_err(|e| e.to_string())?);
    let mesh = qymcad_kernel::import_step(&path.to_string_lossy(), 0.01).map_err(|e| format!("the STEP does not import: {e:?}"))?.remove(0).mesh;
    let body = p.add_mesh(mesh);
    p.import_tree_as_parts(vec![ImportNode { name: "part".into(), body: Some(body), ..Default::default() }], source, "part").ok_or("not brought in")?;
    let part = p.timeline.iter().find(|n| n.kind.owns_body(body)).and_then(|n| n.parent);
    p.set_active_component(part);
    let shapes = qymcad_testkit::restore_import_shapes(&p);
    let (_, shapes) = qymcad_testkit::regenerate_with_shapes(&mut p, shapes);
    Ok(BroughtIn { p, body, shapes })
}

/// The volume node `node` builds, the document rebuilt; the rebuild going red is told.
fn volume(p: &mut Project, shapes: &mut HashMap<Id, Shape>, node: Id) -> Result<f64, String> {
    let (report, sh) = qymcad_testkit::regenerate_with_shapes(p, std::mem::take(shapes));
    *shapes = sh;
    if !report.errors.is_empty() {
        return Err(format!("the rebuild went red: {:?}", report.errors));
    }
    shapes.get(&node).map(|s| s.volume()).ok_or_else(|| "the node is not built".into())
}

fn run(c: &Case) -> Result<(), String> {
    let BroughtIn { mut p, body, mut shapes } = brought_in(c.name, &(c.solid)())?;
    let mi = p.mesh_index(body).ok_or("no mesh")?;
    let f = p.bodies[mi].faces.iter().find(|f| (c.clicked)(f)).ok_or("no face to click")?;
    let key = FaceKey { index: 0, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id };
    let k = OcctKernel { shapes: std::cell::RefCell::new(std::mem::take(&mut shapes)), ..Default::default() };
    let prism = p.prism_at(body, key.id, &k).map_err(|e| format!("not found: {e:?}"))?;
    let sk = p.add_sketch_from_face(FromFace { body, face: key, profile: &prism.profile, outline: &prism.outline, name: "S".into() }, &k).ok_or("no sketch")?;
    shapes = k.shapes.into_inner();
    if p.sketch_dof(sk.si) != (0, 0) {
        return Err(format!("the sketch stands {:?}, not fully defined; {:?}", p.sketch_dof(sk.si), sk.defined));
    }
    let before = shapes.get(&body).map(|s| s.volume()).ok_or("the body is not built")?;
    let node = p.add_redrawn(body, &prism, sk.sketch);
    let redrawn = volume(&mut p, &mut shapes, node)?;
    if (redrawn - before).abs() > 1e-3 * before {
        return Err(format!("redrawn it holds {redrawn} mm^3, not {before}"));
    }
    // the height in the tree drives the body
    p.set_feat_dim(node, "height", c.height.to_string());
    let taller = volume(&mut p, &mut shapes, node)?;
    if (taller - redrawn - c.height_gains).abs() > 1e-3 * before {
        return Err(format!("made {} high it holds {taller} mm^3, not {}", c.height, redrawn + c.height_gains));
    }
    // and a size of the sketch typed anew
    let s = &mut p.sketches[sk.si];
    let size = s.constraints.iter_mut().find_map(|x| match x {
        Constraint::Distance { d, driven: false, .. } | Constraint::Diameter { d, driven: false, .. } if (*d - c.resize.from).abs() < 1e-6 => Some(d),
        _ => None,
    });
    *size.ok_or_else(|| format!("no size of {} in the sketch", c.resize.from))? = c.resize.to;
    p.solve_sketch(sk.si);
    p.mark_sketch_dirty(sk.sketch);
    let resized = volume(&mut p, &mut shapes, node)?;
    if (resized - taller - c.gains).abs() > 1e-3 * before {
        return Err(format!("{} made {} it holds {resized} mm^3, not {}", c.resize.from, c.resize.to, taller + c.gains));
    }
    Ok(())
}

#[test]
fn a_boss_or_a_pocket_is_redrawn_by_its_sketch() {
    let mut wrong = Vec::new();
    for c in cases() {
        if let Err(why) = run(&c) {
            wrong.push(format!("{}: {why}", c.name));
        }
    }
    assert!(wrong.is_empty(), "{} of {} went wrong:\n{}", wrong.len(), cases().len(), wrong.join("\n"));
}
