//! A MESH CLAUDE MENDS IN THE OPEN WINDOW IS ONE STEP OF UNDO, and the window sees the mended mesh: a cube from a file
//! with one triangle wound inward has it turned back, though the mesh keeps its corners and its triangles in number -
//! the document's key counts a mesh by its size - and Ctrl+Z winds it inward again.
use super::*;

/// A cube of side 10, its corners shared, one triangle wound inward.
fn cube_turned() -> qymcad_core::geom::Mesh {
    let c = |i: usize| qymcad_core::geom::Point3::new((i & 1) as f64 * 10.0, ((i >> 1) & 1) as f64 * 10.0, ((i >> 2) & 1) as f64 * 10.0);
    let mut tris = vec![[0, 2, 1], [1, 2, 3], [4, 5, 6], [5, 7, 6], [0, 1, 4], [1, 5, 4], [2, 6, 3], [3, 6, 7], [0, 4, 2], [2, 4, 6], [1, 3, 5], [3, 7, 5]];
    tris[4] = [0, 4, 1];
    qymcad_core::geom::Mesh { verts: (0..8).map(c).collect(), tris, ..Default::default() }
}

/// How many triangles of the window's piece are wound inward.
fn wound_in(app: &App, body: Id) -> usize {
    let mi = app.project.mesh_index(body).expect("the piece stands");
    qymcad_doc::print::mesh_report::mesh_report(&app.project.bodies[mi].mesh).flipped
}

#[test]
fn a_mesh_claude_mends_is_one_step_and_seen() {
    let mut app = App::default();
    app.project.new_document();
    let source = app.project.add_source("cube.stl", vec![1]);
    let body = app.project.add_mesh(cube_turned());
    let piece = qymcad_core::model::ImportNode { name: "cube".into(), body: Some(body), mesh: true, ..Default::default() };
    app.project.import_tree_as_parts(vec![piece], source, "cube").expect("came in");
    app.set.claude_link = ClaudeLink::On;
    assert_eq!(wound_in(&app, body), 1, "setup: one triangle wound inward");
    let folder = place("mend");
    let path = folder.file(SOCKET);
    super::super::listen_at(path.clone());
    let steps = app.disk.edits.undo.len();
    let mut hand = Hand::new(&mut app);
    hand.frame(Vec::new());
    let rev = hand.app.regen.geom_rev;
    let answers = served(&mut hand, claude_says(&path, vec![("mesh_repair", json!({ "body": { "body": body } }))]));
    assert_eq!(reply_of(&answers[0])["ok"], json!(true), "{}", answers[0]);
    assert_ne!(hand.app.regen.geom_rev, rev, "the window did not see its mesh change: what it draws is kept from before");
    drop(hand);
    assert_eq!(wound_in(&app, body), 0, "the window's piece is not mended");
    assert_eq!(app.disk.edits.undo.len(), steps + 1, "the repair is not one step of undo");
    Hand::new(&mut app).undo();
    assert_eq!(wound_in(&app, body), 1, "Ctrl+Z did not take the repair back");
}
