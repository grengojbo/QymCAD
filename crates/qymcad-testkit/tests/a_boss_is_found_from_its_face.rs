//! A BOSS OR A POCKET IS FOUND FROM ITS FLAT FACE on a solid come from a STEP file: the walls round the face square to
//! it, the base they end on, how tall, which way it stands. What cannot be redrawn by one sketch is refused in words: a
//! rim rounded at the face, a face with nothing standing on it, a hole going on into the plate, a face that is not
//! flat. Built with the live kernel.
//!
//! Body x what is clicked: failures are gathered and told together.
use qymcad_core::errors::CoreError;
use qymcad_core::geom::MeshFace;
use qymcad_core::model::{Id, ImportNode, PrismKind, Project, RimBlend, RimCut, Rims};
use qymcad_kernel::specimen::Specimen;
use qymcad_kernel::{OcctKernel, Shape};
use std::collections::HashMap;

fn block(x: f64, y: f64, w: f64, d: f64, z: f64, h: f64) -> Shape {
    let at = [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z];
    qymcad_kernel::specimen::extrude(&[0.0, 0.0, w, 0.0, w, d, 0.0, d], h).expect("a block").moved(&at).expect("placed")
}

fn post(x: f64, y: f64, r: f64, z: f64, h: f64) -> Shape {
    let at = [1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, y, 0.0, 0.0, 1.0, z];
    qymcad_kernel::specimen::cylinder(r, h).expect("a cylinder").moved(&at).expect("placed")
}

/// A plate of 60 x 40 x 5 with a boss of 30 x 20 x 8 from (10, 10) on it.
fn bossed() -> Shape {
    block(0.0, 0.0, 60.0, 40.0, 0.0, 5.0).combined(&block(10.0, 10.0, 30.0, 20.0, 5.0, 8.0), 1).expect("fused")
}

/// The edges of `s` running along `z` from `z0` to `z1` at the corners of the boss of `bossed`.
fn uprights(s: &Shape, z0: f64, z1: f64) -> Vec<u32> {
    s.edges_info()
        .into_iter()
        .filter(|e| {
            let (a, b) = (e.poly.first().copied(), e.poly.last().copied());
            let (Some(a), Some(b)) = (a, b) else { return false };
            let corner = |p: [f32; 3]| [10.0, 40.0].iter().any(|x| (p[0] as f64 - x).abs() < 1e-6) && [10.0, 30.0].iter().any(|y| (p[1] as f64 - y).abs() < 1e-6);
            corner(a) && corner(b) && (a[2] as f64 - z0).abs().min((a[2] as f64 - z1).abs()) < 1e-6 && (a[2] - b[2]).abs() > 1.0
        })
        .map(|e| e.id)
        .collect()
}

/// The edges round the top of the boss of `bossed`, at z = 13.
fn top_rim(s: &Shape) -> Vec<u32> {
    s.edges_info().into_iter().filter(|e| e.poly.iter().all(|p| (p[2] as f64 - 13.0).abs() < 1e-6)).map(|e| e.id).collect()
}

/// What a case expects: the prism found, or the words of a refusal.
enum Want {
    Found { kind: PrismKind, walls: usize, inner: usize, height: f64, rims: Rims },
    Refused(CoreError),
}

const SHARP: Rims = Rims { at_face: None, at_base: None };

/// The edges round the foot of the boss of `bossed`, at z = 5 inside the plate's outline.
fn foot(s: &Shape) -> Vec<u32> {
    s.edges_info().into_iter().filter(|e| e.poly.iter().all(|p| (p[2] as f64 - 5.0).abs() < 1e-6 && p[0] > 1.0 && p[0] < 59.0 && p[1] > 1.0 && p[1] < 39.0)).map(|e| e.id).collect()
}

struct Case {
    name: &'static str,
    solid: fn() -> Shape,
    clicked: fn(&MeshFace) -> bool,
    want: Want,
}

fn up_at(z: f64) -> impl Fn(&MeshFace) -> bool {
    move |f: &MeshFace| f.normal[2] > 0.999 && (f.centroid.z - z).abs() < 1e-6
}

fn cases() -> Vec<Case> {
    vec![
        Case { name: "boss", solid: bossed, clicked: |f| up_at(13.0)(f), want: Want::Found { kind: PrismKind::Boss, walls: 4, inner: 0, height: 8.0, rims: SHARP } },
        Case {
            name: "pocket",
            solid: || block(0.0, 0.0, 60.0, 40.0, 0.0, 10.0).combined(&block(10.0, 10.0, 30.0, 20.0, 6.0, 5.0), 0).expect("cut"),
            clicked: |f| up_at(6.0)(f),
            want: Want::Found { kind: PrismKind::Pocket, walls: 4, inner: 0, height: 4.0, rims: SHARP },
        },
        Case {
            name: "round post",
            solid: || block(0.0, 0.0, 60.0, 40.0, 0.0, 5.0).combined(&post(30.0, 20.0, 6.0, 5.0, 8.0), 1).expect("fused"),
            clicked: |f| up_at(13.0)(f),
            want: Want::Found { kind: PrismKind::Boss, walls: 1, inner: 0, height: 8.0, rims: SHARP },
        },
        Case {
            name: "boss with its corners rounded",
            solid: || {
                let s = bossed();
                let e = uprights(&s, 5.0, 13.0);
                s.filleted(3.0, &e).expect("rounded")
            },
            clicked: |f| up_at(13.0)(f),
            want: Want::Found { kind: PrismKind::Boss, walls: 8, inner: 0, height: 8.0, rims: SHARP },
        },
        Case {
            name: "boss drilled to its base",
            solid: || bossed().combined(&post(25.0, 20.0, 3.0, 5.0, 9.0), 0).expect("drilled"),
            clicked: |f| up_at(13.0)(f),
            want: Want::Found { kind: PrismKind::Boss, walls: 4, inner: 1, height: 8.0, rims: SHARP },
        },
        Case {
            name: "boss rounded at its top",
            solid: || {
                let s = bossed();
                let e = top_rim(&s);
                s.filleted(2.0, &e).expect("rounded")
            },
            clicked: |f| up_at(13.0)(f),
            want: Want::Found { kind: PrismKind::Boss, walls: 4, inner: 0, height: 8.0, rims: Rims { at_face: Some(RimBlend { cut: RimCut::Round, size: 2.0 }), at_base: None } },
        },
        Case {
            name: "boss cut at its top",
            solid: || {
                let s = bossed();
                let e = top_rim(&s);
                s.chamfered(1.5, &e).expect("cut")
            },
            clicked: |f| up_at(13.0)(f),
            want: Want::Found { kind: PrismKind::Boss, walls: 4, inner: 0, height: 8.0, rims: Rims { at_face: Some(RimBlend { cut: RimCut::Chamfer, size: 1.5 }), at_base: None } },
        },
        Case {
            name: "boss rounded at its foot",
            solid: || {
                let s = bossed();
                let e = foot(&s);
                s.filleted(1.0, &e).expect("rounded")
            },
            clicked: |f| up_at(13.0)(f),
            want: Want::Found { kind: PrismKind::Boss, walls: 4, inner: 0, height: 8.0, rims: Rims { at_face: None, at_base: Some(RimBlend { cut: RimCut::Round, size: 1.0 }) } },
        },
        Case {
            name: "boss rounded along one edge of its top",
            solid: || {
                let s = bossed();
                let e = top_rim(&s);
                s.filleted(2.0, &e[..1]).expect("rounded")
            },
            clicked: |f| up_at(13.0)(f),
            want: Want::Refused(CoreError::PrismRimRounded),
        },
        Case { name: "plain plate", solid: || block(0.0, 0.0, 60.0, 40.0, 0.0, 5.0), clicked: |f| up_at(5.0)(f), want: Want::Refused(CoreError::NotAPrism) },
        Case {
            name: "boss drilled through the plate",
            solid: || bossed().combined(&post(25.0, 20.0, 3.0, -1.0, 15.0), 0).expect("drilled"),
            clicked: |f| up_at(13.0)(f),
            want: Want::Refused(CoreError::PrismHoleDeeper),
        },
        Case {
            name: "the side of a post",
            solid: || block(0.0, 0.0, 60.0, 40.0, 0.0, 5.0).combined(&post(30.0, 20.0, 6.0, 5.0, 8.0), 1).expect("fused"),
            clicked: |f| f.normal[2].abs() < 0.5 && ((f.centroid.x - 30.0).hypot(f.centroid.y - 20.0)) < 6.5 && f.centroid.z > 5.5,
            want: Want::Refused(CoreError::PrismFaceNotFlat),
        },
    ]
}

/// A document with a solid brought in: the body and the live shapes the rebuild left.
struct BroughtIn {
    p: Project,
    body: Id,
    shapes: HashMap<Id, Shape>,
}

/// The solid written to a STEP file and brought in as a person brings one: a body with no recipe, its faces numbered
/// by the file.
fn brought_in(name: &str, s: &Shape) -> Result<BroughtIn, String> {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/prism-found"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.step", name.replace(' ', "-")));
    qymcad_kernel::write_step(&[(s, qymcad_core::feature::PLACE_IDENTITY)], &path.to_string_lossy()).map_err(|e| format!("the STEP is not written: {e:?}"))?;
    let mut p = Project::default();
    p.new_document();
    let source = p.add_source("part.step", std::fs::read(&path).map_err(|e| e.to_string())?);
    let mesh = qymcad_kernel::import_step(&path.to_string_lossy(), 0.01).map_err(|e| format!("the STEP does not import: {e:?}"))?.remove(0).mesh;
    let body = p.add_mesh(mesh);
    p.import_tree_as_parts(vec![ImportNode { name: "part".into(), body: Some(body), ..Default::default() }], source, "part").ok_or("not brought in")?;
    let shapes = qymcad_testkit::restore_import_shapes(&p);
    let (_, shapes) = qymcad_testkit::regenerate_with_shapes(&mut p, shapes);
    Ok(BroughtIn { p, body, shapes })
}

fn run(c: &Case) -> Result<(), String> {
    let BroughtIn { p, body, shapes } = brought_in(c.name, &(c.solid)())?;
    let mi = p.mesh_index(body).ok_or("no mesh")?;
    let face = p.bodies[mi].faces.iter().find(|f| (c.clicked)(f)).ok_or("no face to click")?.id;
    let k = OcctKernel { shapes: std::cell::RefCell::new(shapes), ..Default::default() };
    let got = p.prism_at(body, face, &k);
    match (&c.want, got) {
        (Want::Found { kind, walls, inner, height, rims }, Ok(found)) => {
            let rim_like = |a: Option<RimBlend>, b: Option<RimBlend>| a.map_or(b.is_none(), |a| b.is_some_and(|b| a.cut == b.cut && (a.size - b.size).abs() < 1e-3));
            let rims_ok = rim_like(found.rims.at_face, rims.at_face) && rim_like(found.rims.at_base, rims.at_base);
            if found.kind != *kind || found.walls.len() != *walls || found.inner.len() != *inner || (found.height - height).abs() > 1e-6 || found.base.is_empty() || !rims_ok {
                return Err(format!("found {found:?}"));
            }
            Ok(())
        }
        (Want::Found { .. }, Err(e)) => Err(format!("refused: {e:?}")),
        (Want::Refused(want), Ok(found)) => Err(format!("found {found:?}, not refused {want:?}")),
        (Want::Refused(want), Err(e)) if format!("{e:?}") == format!("{want:?}") => Ok(()),
        (Want::Refused(want), Err(e)) => Err(format!("refused {e:?}, not {want:?}")),
    }
}

#[test]
fn a_boss_or_a_pocket_is_found_from_its_face() {
    let mut wrong = Vec::new();
    for c in cases() {
        if let Err(why) = run(&c) {
            wrong.push(format!("{}: {why}", c.name));
        }
    }
    assert!(wrong.is_empty(), "{} of {} went wrong:\n{}", wrong.len(), cases().len(), wrong.join("\n"));
}
