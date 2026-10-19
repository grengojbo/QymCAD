//! EVERY HOLE, ROUNDING AND CHAMFER OF A BODY IS MADE AGAIN EXACT BY ONE NODE, through the document: the body comes in
//! as a person brings it - a STEP, or a mesh made a polyhedron or recognised into exact surfaces - everything on it is
//! found, the features go in groups of one size, and one node takes all their faces off and lays them down again. The
//! body then holds what the exact solid held; a size of a group edited in the tree drives every feature of the group,
//! as the exact solid built with that size; a group left out stays as it was. Built with the live kernel.
//!
//! The solids: a plate drilled 4 blind, 5 and 6 through, one corner rounded 4 and another cut back 3; a block whose four
//! upright edges are rounded 4 and whose top edge is then rounded 2 all round, drilled 6 - a chain of roundings, which
//! no rounding of it taken off alone lets go of.
//!
//! Solid x body x edit: failures are gathered and told together.
use qymcad_core::model::{Id, ImportNode, Project, RemakeGroup};
use qymcad_kernel::specimen::{self, Specimen};
use qymcad_kernel::Shape;
use qymcad_meshfit::feature::{features_of, grouped};

/// The sizes a solid is built with: the bore drilled 5 in the plate, the rounding of 2 in the chain.
#[derive(Clone, Copy)]
struct Sizes {
    bore: f64,
    small_round: f64,
}

const DRAWN: Sizes = Sizes { bore: 5.0, small_round: 2.0 };

fn placed(s: &Shape, by: [f64; 3]) -> Shape {
    specimen::placed(s, &[1.0, 0.0, 0.0, by[0], 0.0, 1.0, 0.0, by[1], 0.0, 0.0, 1.0, by[2]]).expect("placed")
}

/// A drill of radius `r` at (x, y), from z = `from` up `h`.
struct Drill {
    x: f64,
    y: f64,
    r: f64,
    from: f64,
    h: f64,
}

fn bore(d: Drill) -> Shape {
    placed(&specimen::cylinder(d.r, d.h).expect("a cylinder"), [d.x, d.y, d.from])
}

fn block() -> Shape {
    specimen::extrude(&[0.0, 0.0, 60.0, 0.0, 60.0, 40.0, 0.0, 40.0], 10.0).expect("a block")
}

fn upright(s: &Shape, (x, y): (f64, f64)) -> u32 {
    s.edges_info().into_iter().find(|e| e.poly.iter().all(|p| (p[0] as f64 - x).abs() < 1e-6 && (p[1] as f64 - y).abs() < 1e-6)).map(|e| e.id).expect("the upright edge")
}

fn plate(z: Sizes) -> Shape {
    let p = block();
    let p = p.filleted(4.0, &[upright(&p, (60.0, 40.0))]).expect("rounded");
    let p = p.chamfered(3.0, &[upright(&p, (0.0, 0.0))]).expect("cut");
    [
        Drill { x: 15.0, y: 20.0, r: 3.0, from: -1.0, h: 12.0 },
        Drill { x: 30.0, y: 10.0, r: z.bore / 2.0, from: -1.0, h: 12.0 },
        Drill { x: 45.0, y: 20.0, r: 2.0, from: 5.0, h: 6.0 },
    ]
    .into_iter()
    .fold(p, |p, d| p.combined(&bore(d), 0).expect("drilled"))
}

fn chained(z: Sizes) -> Shape {
    let p = block();
    let ups: Vec<u32> = [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)].iter().map(|&at| upright(&p, at)).collect();
    let p = p.filleted(4.0, &ups).expect("rounded upright");
    let top: Vec<u32> = p.edges_info().into_iter().filter(|e| e.poly.iter().all(|q| (q[2] as f64 - 10.0).abs() < 1e-6)).map(|e| e.id).collect();
    let p = p.filleted(z.small_round, &top).expect("rounded round the top");
    p.combined(&bore(Drill { x: 15.0, y: 20.0, r: 3.0, from: -1.0, h: 12.0 }), 0).expect("drilled")
}

/// How the solid comes in.
#[derive(Clone, Copy, Debug)]
enum Came {
    Step,
    Polyhedron,
    Recognised,
}

/// The document a solid came into, and the body to find its features on.
struct Brought {
    p: Project,
    body: Id,
    shapes: std::collections::HashMap<Id, Shape>,
}

fn brought(name: &str, s: &Shape, came: Came) -> Result<Brought, String> {
    let mut p = Project::default();
    p.new_document();
    let src = match came {
        Came::Step => {
            let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/made-exact-at-once"));
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(format!("{name}.step"));
            qymcad_kernel::write_step_of(s, &path.to_string_lossy()).map_err(|e| format!("the STEP is not written: {e:?}"))?;
            let source = p.add_source("part.step", std::fs::read(&path).map_err(|e| e.to_string())?);
            let mesh = qymcad_kernel::import_step(&path.to_string_lossy(), 0.01).map_err(|e| format!("the STEP does not import: {e:?}"))?.remove(0).mesh;
            let body = p.add_mesh(mesh);
            p.import_tree_as_parts(vec![ImportNode { name: "part".into(), body: Some(body), ..Default::default() }], source, "part").ok_or("not brought in")?;
            body
        }
        Came::Polyhedron | Came::Recognised => {
            let mesh = specimen::mesh_merged(s, 0.01).ok_or("no mesh")?.mesh;
            let source = p.add_source("part.stl", vec![1]);
            let body = p.add_mesh(mesh);
            p.import_tree_as_parts(vec![ImportNode { name: "part".into(), body: Some(body), mesh: true, ..Default::default() }], source, "part").ok_or("not brought in")?;
            body
        }
    };
    let part = p.timeline.iter().find(|n| n.kind.owns_body(src)).and_then(|n| n.parent);
    p.set_active_component(part);
    let body = match came {
        Came::Step => src,
        Came::Polyhedron => p.add_mesh_solid(src, 0.0),
        Came::Recognised => p.add_mesh_recognised(src, 1.0, 30.0, 0.0),
    };
    let shapes = qymcad_testkit::restore_import_shapes(&p);
    let (report, shapes) = qymcad_testkit::regenerate_with_shapes(&mut p, shapes);
    if !report.errors.is_empty() {
        return Err(format!("the body did not come in: {:?}", report.errors));
    }
    Ok(Brought { p, body, shapes })
}

/// The groups found on the body, as the window finds them: the body's own mesh and faces.
fn groups_on(b: &Brought) -> Result<Vec<RemakeGroup>, String> {
    let mi = b.p.mesh_index(b.body).ok_or("the body has no mesh")?;
    let (mesh, faces) = (&b.p.bodies[mi].mesh, &b.p.bodies[mi].faces);
    Ok(grouped(&features_of(mesh, faces, 1.0).found, faces))
}

/// One case: the solid, the ways it comes in, how many groups and features it gives, the size edited and what it then
/// is built with.
struct Case {
    name: &'static str,
    solid: fn(Sizes) -> Shape,
    came: &'static [Came],
    groups: usize,
    features: usize,
    edit: &'static str,
    edited: Sizes,
}

fn cases() -> Vec<Case> {
    vec![
        // the groups: hole 4 blind, hole 5, hole 6, chamfer 3, rounding 4 - the bore of 5 is the second
        Case { name: "plate", solid: plate, came: &[Came::Step, Came::Polyhedron, Came::Recognised], groups: 5, features: 5, edit: "diameter.2", edited: Sizes { bore: 5.5, ..DRAWN } },
        // the groups: hole 6, rounding 2, rounding 4. Recognised, the chain leaves a side of the block in loose triangles
        // beside a corner, and a rounding takes one for a side: the group that does not lay is the tool's to find and
        // leave out, with its reason
        Case { name: "chain", solid: chained, came: &[Came::Step, Came::Polyhedron], groups: 3, features: 9, edit: "radius.2", edited: Sizes { small_round: 1.5, ..DRAWN } },
    ]
}

/// A document rebuilt: what the node holds, and the live bodies to rebuild from next.
struct Rebuilt {
    volume: f64,
    shapes: std::collections::HashMap<Id, Shape>,
}

fn volume_of(p: &mut Project, shapes: std::collections::HashMap<Id, Shape>, node: Id) -> Result<Rebuilt, String> {
    let (report, shapes) = qymcad_testkit::regenerate_with_shapes(p, shapes);
    if !report.errors.is_empty() {
        return Err(format!("the rebuild went red: {:?}", report.errors));
    }
    let volume = shapes.get(&node).map(|s| s.volume()).ok_or("the node is not built")?;
    Ok(Rebuilt { volume, shapes })
}

/// Whether the volume built is the one wanted, to a ten-thousandth of it.
fn near(built: &Rebuilt, want: f64) -> bool {
    (built.volume - want).abs() <= 1e-4 * want
}

/// What a case came to: the volume of the body after the size was edited.
struct Checked {
    edited: f64,
}

fn run(c: &Case, came: Came) -> Result<Checked, String> {
    let exact = (c.solid)(DRAWN);
    let mut b = brought(&format!("{}-{came:?}", c.name), &exact, came)?;
    let groups = groups_on(&b)?;
    let count: usize = groups.iter().map(|g| g.items.len()).sum();
    if groups.len() != c.groups || count != c.features {
        return Err(format!("{} groups of {count} features found, not {} of {}", groups.len(), c.groups, c.features));
    }
    let before = b.p.timeline.len();
    let node = b.p.add_remade_features(b.body, groups);
    if b.p.timeline.len() != before + 1 {
        return Err(format!("made exact in {} nodes, not one", b.p.timeline.len() - before));
    }
    let made = volume_of(&mut b.p, b.shapes, node)?;
    if !near(&made, exact.volume()) {
        return Err(format!("made exact it holds {} mm^3, the solid {}", made.volume, exact.volume()));
    }
    b.p.set_feat_dim(
        node,
        c.edit,
        match c.edit {
            "diameter.2" => c.edited.bore.to_string(),
            _ => c.edited.small_round.to_string(),
        },
    );
    let want = (c.solid)(c.edited).volume();
    let edited = volume_of(&mut b.p, made.shapes, node)?;
    if !near(&edited, want) {
        return Err(format!("{} edited holds {} mm^3, the solid built so {want}", c.edit, edited.volume));
    }
    Ok(Checked { edited: edited.volume })
}

#[test]
fn every_feature_of_a_body_is_made_exact_by_one_node_and_a_group_edited_drives_it() {
    let mut wrong = Vec::new();
    let mut tried = 0;
    for c in cases() {
        for &came in c.came {
            tried += 1;
            if let Err(why) = run(&c, came).map(|done| done.edited) {
                wrong.push(format!("{} / {came:?}: {why}", c.name));
            }
        }
    }
    assert!(wrong.is_empty(), "{} of {tried} went wrong:\n{}", wrong.len(), wrong.join("\n"));
}

#[test]
fn a_group_left_out_stays_as_it_was() {
    let exact = plate(DRAWN);
    let mut b = brought("plate-but-the-chamfer", &exact, Came::Step).expect("the plate");
    let groups: Vec<RemakeGroup> = groups_on(&b).expect("the groups").into_iter().filter(|g| !g.items.iter().any(|r| matches!(r.exact, qymcad_core::model::Exact::Chamfer { .. }))).collect();
    assert_eq!(groups.len(), 4);
    let node = b.p.add_remade_features(b.body, groups);
    let made = volume_of(&mut b.p, b.shapes, node).expect("built");
    assert!(near(&made, exact.volume()), "{} against {}", made.volume, exact.volume());
    let faces = b.p.regen_faces.get(&node).cloned().unwrap_or_default();
    let slanted = faces.iter().filter(|f| f.normal[0] < -0.5 && f.normal[1] < -0.5).count();
    assert_eq!(slanted, 1, "the chamfer left out is still on the body");
}
