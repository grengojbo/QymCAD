//! CLAUDE IN THE OPEN WINDOW: the calls the program Claude starts sends over `qymcad-bridge` are done here, on the
//! document a person is looking at, with the same tools the program would run on a document of its own.
//!
//! Once a frame the window takes at most one call. It takes none while a person holds the document - an operation of
//! theirs is open, or a rebuild is running - and a call left waiting past `TAKE_WITHIN` is withdrawn by the channel and
//! Claude told the window is busy: a call must never land in the middle of a person's drag.
//!
//! A CALL IS ONE STEP OF UNDO, named for what it did ("Claude: Fillet"). The document and its live bodies are lent to
//! the tool as they stand - nothing is built again but what the call changes - and handed back; a call that changed
//! nothing leaves no step.
//!
//! THE STATE LIVES HERE AND NOT ON `App`, per thread: the window has one thread, and each check runs on its own with a
//! window and a socket of its own.

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use qymcad_bridge::{Call, Listener, Wait};
use qymcad_tools::picture::Eye;
use qymcad_tools::tool::{After, Ctx, InWindow, Refusal, Seen, Stage};
use qymcad_tools::tools::selection::{End as Tip, Picked};
use qymcad_ui_state::{ClaudeLink, PartCtx};
use serde_json::Value;

/// HOW LONG A CALL MAY WAIT for the window to take it. A person's drag or a rebuild holds the document; past this the
/// call is withdrawn and Claude is told the window is busy.
const TAKE_WITHIN: Duration = Duration::from_secs(5);

/// The window's end of the channel, as far as it got.
enum End {
    /// Not open: the setting is off, or nothing was tried yet.
    Closed,
    Open(Listener),
    /// Opening was refused; said once in the status line and not tried again until the setting is turned off and on.
    Refused,
}

thread_local! {
    static END: RefCell<End> = const { RefCell::new(End::Closed) };
    /// WHERE CLAUDE WORKS: the part its calls lay new nodes in, kept apart from where the person is looking. A call
    /// that makes a part (a sketch in an assembly) or steps into one works there from then on; the window does not
    /// follow it, and the next call does not lose it to the window's own context, which every frame lays back on the
    /// document. Reported behaviour, found by the live chain: a sketch made its part, the next call - a rounding with no
    /// body named - looked for "part 1", the assembly the window stood at, and was refused.
    static WORKS_IN: std::cell::Cell<Option<qymcad_core::model::Id>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
thread_local! {
    /// A socket of the check's own, in place of the one in the program's folder.
    static PLACE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Listen at `path` instead of the program's own socket, for the rest of this thread.
#[cfg(test)]
pub(crate) fn listen_at(path: PathBuf) {
    PLACE.with_borrow_mut(|p| *p = Some(path));
}

fn place() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(p) = PLACE.with_borrow(Clone::clone) {
        return Some(p);
    }
    qymcad_bridge::default_path()
}

/// ONE FRAME'S TURN: open or close the window's end as the setting says, and do at most one call.
pub(crate) fn pump(pc: &mut PartCtx, ctx: &egui::Context) {
    let Some(call) = take(pc, ctx) else { return };
    let reply = run(pc, &call.tool, call.arguments.clone());
    call.answer(reply);
    // the document changed under the frame that is drawing: the next one shows it
    ctx.request_repaint();
}

/// The next call to do, opening or closing the window's end first.
fn take(pc: &mut PartCtx, ctx: &egui::Context) -> Option<Call> {
    END.with_borrow_mut(|end| {
        match (pc.set.claude_link, &*end) {
            (ClaudeLink::Off, End::Closed) => return None,
            (ClaudeLink::Off, _) => {
                *end = End::Closed; // dropping the listener removes the socket
                return None;
            }
            (ClaudeLink::On, End::Closed) => *end = open(pc, ctx),
            (ClaudeLink::On, _) => {}
        }
        let End::Open(listener) = &*end else { return None };
        if holds_the_document(pc) {
            return None;
        }
        listener.next()
    })
}

fn open(pc: &mut PartCtx, ctx: &egui::Context) -> End {
    let refused = |why: String, pc: &mut PartCtx| {
        *pc.status = qymcad_i18n::tr1("bridge-cannot-open", "why", &why);
        End::Refused
    };
    let Some(path) = place() else { return refused("no folder for the program's files".into(), pc) };
    let waker = ctx.clone();
    match Listener::open(&path, Wait { answer_within: TAKE_WITHIN }, std::sync::Arc::new(move || waker.request_repaint())) {
        Ok(listener) => End::Open(listener),
        Err(e) => refused(format!("{e:?}"), pc),
    }
}

/// A PERSON HOLDS THE DOCUMENT: an operation of theirs is open (a drag, a sketch being edited, a tool in hand), or a
/// rebuild is running on it.
fn holds_the_document(pc: &PartCtx) -> bool {
    pc.edits.open.is_some() || pc.regen.regen_running()
}

/// WHAT THE PERSON HAS SELECTED, by the keys the document keeps it under: the one thing the window marks as selected,
/// and the faces gathered for a tool, when there are any.
fn picked(pc: &PartCtx) -> Vec<Picked> {
    use qymcad_ui_state::Sel;
    let p = &*pc.project;
    let one = match *pc.sel {
        Sel::None => None,
        Sel::Mesh(mi) => p.mesh_id(mi).map(Picked::Body),
        Sel::Face(mi, fi) => p.mesh_id(mi).zip(p.bodies.get(mi).and_then(|b| b.faces.get(fi))).map(|(body, f)| Picked::Face { body, face: f.id }),
        Sel::Contour(i) => p.contour_id(i).map(Picked::Contour),
        Sel::Sketch(i) => p.sketches.get(i).map(|s| Picked::Sketch(s.id)),
        Sel::Plane(i) => p.planes.get(i).map(|w| Picked::Plane(w.id)),
        Sel::DatumPoint(i) => p.datum_points.get(i).map(|d| Picked::Point(d.id)),
        Sel::DatumAxis(i) => p.datum_axes.get(i).map(|d| Picked::Axis(d.id)),
        Sel::Feature(i) => p.timeline.get(i).map(|n| Picked::Feature(n.id)),
        Sel::Component(i) => p.components.get(i).map(|c| Picked::Part(c.id)),
        Sel::Joint(id) => Some(Picked::Joint(id)),
        Sel::Edge(body, edge) => Some(Picked::Edge { body, edge }),
        Sel::Vertex(body, edge, far) => Some(Picked::Corner { body, edge, end: if far { Tip::B } else { Tip::A } }),
    };
    let mut out: Vec<Picked> = one.into_iter().collect();
    if let Some(body) = pc.gsel.faces_body {
        let mut faces: Vec<u32> = pc.gsel.faces.iter().copied().collect();
        faces.sort_unstable();
        for face in faces {
            let f = Picked::Face { body, face };
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

/// The tools that act on the window itself rather than on its document; a person does these from the window.
const THE_WINDOWS_OWN: [&str; 5] = ["new_project", "open_project", "save_project", "undo", "redo"];

/// DO ONE CALL on the window's document, as one step of undo.
fn run(pc: &mut PartCtx, tool: &str, arguments: Value) -> Value {
    if THE_WINDOWS_OWN.contains(&tool) {
        let refusal = Refusal::new("window-own", &format!("In the open window {tool} is the person's to do, from the window's own menu."), Stage::Window)
            .with_hint("Ask the person to do it in QymCAD, then read get_document.");
        return qymcad_tools::tool::refused_reply(tool, &refusal, After::Untouched);
    }
    // the step is named for the tool now and for what it did once that is known
    // what the person has selected is read by its indices, before the document is lent away; and the eye they look through
    let seen = Seen::Window(InWindow { picked: picked(pc), eye: Eye { cam: *pc.cam, projection: pc.set.projection } });
    qymcad_ui_state::begin_edit(pc.edits, pc.project, qymcad_i18n::tr1("bridge-step", "what", tool));
    // the person's context goes back on the document after the call; Claude's own, while its part still stands, is
    // what the call works in
    let window_at = pc.project.active_component;
    if let Some(part) = WORKS_IN.get().filter(|p| pc.project.components.iter().any(|c| c.id == *p)) {
        pc.project.set_active_component(Some(part));
    }
    let lent = qymcad_doc::Lent {
        project: std::mem::take(pc.project),
        shapes: std::mem::take(&mut pc.live.shapes),
        shelved: std::mem::take(&mut pc.live.shelved),
        shelved_sources: std::mem::take(&mut pc.live.shelved_sources),
        params_seen: std::mem::take(pc.params_seen),
    };
    // THE ANSWER IS ENGLISH, as the program's own answers are; the window's language is the person's, and comes back
    let person = qymcad_i18n::language();
    qymcad_i18n::set_language("en");
    let mut ctx = Ctx { doc: qymcad_doc::DocEngine::lend(lent), path: None, seen, language: person.clone() };
    let reply = qymcad_tools::channel::answer(&mut ctx, tool, arguments);
    let step: Option<String> = ctx.doc.history().undo_names().last().map(|s| s.to_string());
    qymcad_i18n::set_language(&person);
    let back = ctx.doc.hand_back();
    *pc.project = back.project;
    WORKS_IN.set(pc.project.active_component);
    pc.project.set_active_component(window_at);
    pc.live.shapes = back.shapes;
    pc.live.shelved = back.shelved;
    pc.live.shelved_sources = back.shelved_sources;
    *pc.params_seen = back.params_seen;
    // the faces kept by body are what a change of topology lays back, and the blobs kept for writing are of the
    // bodies as they were: both are taken from the document as it now stands
    pc.live.faces = pc.project.bodies.iter().filter(|b| !b.faces.is_empty()).map(|b| (b.id, b.faces.clone())).collect();
    pc.live.blobs.clear();
    if let (Some(key), Some((name, _))) = (step, pc.edits.open.as_mut()) {
        *name = qymcad_i18n::tr1("bridge-step", "what", &qymcad_i18n::tr(&key));
    }
    let changed = pc.edits.open.as_ref().is_some_and(|(_, before)| qymcad_ui_state::doc_key(&before.project) != qymcad_ui_state::doc_key(pc.project));
    qymcad_ui_state::commit_edit_if_changed(&mut pc.rebuild());
    if changed {
        qymcad_part::resync_after_topology_change(pc);
        // WHAT CLAUDE DID IS SAID where the window says what happened: a body that appears with no hand on the mouse is
        // otherwise a mystery, and the line names the step Ctrl+Z takes back
        if let Some(last) = pc.edits.undo.last() {
            *pc.status = last.name.clone();
        }
    }
    reply
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::hand::Hand;
    use super::super::App;
    use qymcad_bridge::Link;
    use qymcad_core::feature::{AnchorRef, BasePlane, JointKind};
    use qymcad_core::model::Id;
    use qymcad_ui_state::ClaudeLink;
    use serde_json::{json, Value};
    use std::path::PathBuf;
    use std::time::Duration;

    /// A socket of the check's own, in the temporary folder: a checkout shared into a virtual machine refuses to hold
    /// one.
    pub(crate) fn place(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("qymcad-window").join(format!("{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder for the check");
        dir.join("mcp.sock")
    }

    /// CLAUDE, on a thread of its own: one call, and its answer when it comes.
    fn claude(path: &std::path::Path, tool: &'static str, arguments: Value) -> std::thread::JoinHandle<Value> {
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let mut link = Link::connect(&path, Duration::from_secs(60)).unwrap_or_else(|e| panic!("Claude did not reach the window: {e:?}"));
            link.call(tool, arguments).unwrap_or_else(|e| panic!("the window did not answer: {e:?}"))
        })
    }

    pub(crate) fn reply_of(answer: &Value) -> &Value {
        &answer["structuredContent"]
    }

    fn boxes(app: &App) -> usize {
        app.project.timeline.iter().filter(|n| qymcad_doc::report::kind_of(&n.kind).starts_with("Box")).count()
    }

    /// A CALL FROM CLAUDE LANDS IN THE OPEN WINDOW AS ONE STEP OF UNDO, named for what it did in the person's language,
    /// and Ctrl+Z takes it back - all through whole frames of the window.
    #[test]
    fn a_call_from_claude_is_one_step_of_undo() {
        let mut app = crate::gui::screen_keys::tests::populated();
        app.set.claude_link = ClaudeLink::On;
        let path = place("step");
        super::listen_at(path.clone());
        let (before, steps) = (boxes(&app), app.disk.edits.undo.len());
        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new()); // the window opens its end of the channel
        let job = claude(&path, "box", json!({ "x": 10, "y": 10, "z": 10 }));
        for _ in 0..500 {
            if job.is_finished() {
                break;
            }
            hand.frame(Vec::new());
            std::thread::sleep(Duration::from_millis(10));
        }
        let answer = job.join().expect("Claude's thread ends");
        assert_eq!(reply_of(&answer)["ok"], json!(true), "{answer}");
        drop(hand);
        assert_eq!(boxes(&app), before + 1, "the box did not land in the window's document");
        assert_eq!(app.disk.edits.undo.len(), steps + 1, "the call is not one step of undo");
        let want = qymcad_i18n::tr1("bridge-step", "what", &qymcad_i18n::tr("cmd-box"));
        assert_eq!(app.disk.edits.undo.last().map(|s| s.name.as_str()), Some(want.as_str()), "the step is not named for what Claude did");
        assert_eq!(app.status, want, "the status line does not say what Claude did");

        Hand::new(&mut app).undo();
        assert_eq!(boxes(&app), before, "Ctrl+Z did not take Claude's box back");
    }

    /// A FEATURE CLAUDE CHANGES IS SEEN CHANGED BY THE WINDOW: the faces the window lays back after a change of topology
    /// are the ones the change built, not the ones it had before. An extrusion made 3 mm taller has its top face 3 mm
    /// higher in the window's own document.
    #[test]
    fn a_feature_claude_changes_is_seen_changed() {
        let mut app = crate::gui::screen_keys::tests::populated();
        app.set.claude_link = ClaudeLink::On;
        let path = place("edit");
        super::listen_at(path.clone());
        let node = app.project.timeline.iter().find(|n| qymcad_doc::report::kind_of(&n.kind).starts_with("Extrude")).expect("the plate's extrusion");
        let (key, body) = (node.id, node.kind.body().expect("the extrusion makes a body"));
        let top = |app: &App| app.project.bodies.iter().find(|b| b.id == body).and_then(|b| b.faces.iter().map(|f| f.centroid.z).max_by(f64::total_cmp)).expect("the body has faces");
        let height = app.project.timeline.iter().find(|n| n.id == key).and_then(|n| n.kind.dims().into_iter().find(|(k, _)| *k == "height").map(|(_, v)| v)).expect("the extrusion has a height");
        let before = top(&app);
        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new());
        let job = claude(&path, "edit_feature", json!({ "feature": key, "values": { "height": height + 3.0 } }));
        for _ in 0..500 {
            if job.is_finished() {
                break;
            }
            hand.frame(Vec::new());
            std::thread::sleep(Duration::from_millis(10));
        }
        let answer = job.join().expect("Claude's thread ends");
        assert_eq!(reply_of(&answer)["ok"], json!(true), "{answer}");
        drop(hand);
        let after = top(&app);
        assert!((after - before - 3.0).abs() < 1e-6, "the window shows the top face at {after}, not 3 mm above {before}");
    }

    /// TWO PARTS ON A SLIDER, looked at from the root; the body of the part that slides.
    fn a_slider(app: &mut App) -> Id {
        let before: Vec<Id> = app.project.bodies.iter().map(|b| b.id).collect();
        for k in 0..2 {
            crate::gui::joint_flow::tests::add_part_at(app, k as f64 * 60.0);
        }
        let root = app.project.root;
        app.enter_component(root);
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        crate::gui::commands::refresh_edges(&mut app.part_ctx());
        let mine: Vec<Id> = app.project.bodies.iter().map(|b| b.id).filter(|b| !before.contains(b)).collect();
        let comps: Vec<Id> = mine.iter().map(|b| app.project.body_owner(*b).expect("the owner of the body")).collect();
        app.project.set_grounded(comps[0], true);
        let a = app.project.add_connector(comps[0], AnchorRef::BasePlane(BasePlane::YZ));
        let b = app.project.add_connector(comps[1], AnchorRef::BasePlane(BasePlane::YZ));
        app.project.add_joint(a, b, JointKind::Slider);
        app.project.solve_joints();
        qymcad_ui_state::rebuild_if_dirty(&mut app.rebuild_ctx());
        crate::gui::commands::refresh_edges(&mut app.part_ctx());
        let _ = Hand::new(app).look_at([60.0, 10.0, 5.0], 4.0);
        app.workbench = super::super::Workbench::Assembly;
        mine[1]
    }

    /// NO CALL LANDS IN THE MIDDLE OF A PERSON'S DRAG. While the hand holds a part, the window takes nothing; the
    /// call waits, and is done the moment the part is let go.
    #[test]
    fn the_window_waits_for_the_hand() {
        let mut app = App::default();
        let body = a_slider(&mut app);
        app.set.claude_link = ClaudeLink::On;
        let path = place("hand");
        super::listen_at(path.clone());
        let ctx = egui::Context::default();
        super::pump(&mut app.part_ctx(), &ctx); // the window opens its end of the channel

        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(900.0, 700.0));
        let basis = app.viewing.cam.basis();
        let wt = app.project.body_display_transform(body, qymcad_ui_state::current_ctx_id(&app.active_path, &app.project));
        let top = app.project.regen_faces.get(&body).and_then(|fs| fs.iter().max_by(|a, b| a.centroid.z.total_cmp(&b.centroid.z))).expect("the body has faces");
        let world = qymcad_core::feature::apply12(&wt, [top.centroid.x, top.centroid.y, top.centroid.z]);
        let at = qymcad_ui_state::Screen { cam: &app.viewing.cam, set: &app.set, rect, basis: &basis }.at(world).0;
        let by = egui::vec2(60.0, 0.0);
        assert!(app.joint_grab_part_at(rect, at, by, &basis), "setup: the part must be grabbable");
        qymcad_assembly::joint_giz_drag_to(&mut app.joint_ctx(), at + by / 2.0, by / 2.0, rect, &basis);
        assert!(app.disk.edits.open.is_some(), "setup: the drag holds no operation open");

        let before = boxes(&app);
        let job = claude(&path, "box", json!({ "x": 5, "y": 5, "z": 5 }));
        for _ in 0..50 {
            super::pump(&mut app.part_ctx(), &ctx);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!job.is_finished(), "a call was answered in the middle of the drag");
        assert_eq!(boxes(&app), before, "a call landed in the middle of the drag");

        qymcad_assembly::joint_giz_end_for_test(&mut app.joint_ctx());
        for _ in 0..200 {
            if job.is_finished() {
                break;
            }
            super::pump(&mut app.part_ctx(), &ctx);
            std::thread::sleep(Duration::from_millis(10));
        }
        let answer = job.join().expect("Claude's thread ends");
        assert_eq!(reply_of(&answer)["ok"], json!(true), "the call was not done once the part was let go: {answer}");
        assert_eq!(boxes(&app), before + 1);
    }

    /// SWITCHED OFF, THE WINDOW OFFERS NOTHING: no socket to connect to, and none left behind when it is turned off.
    #[test]
    fn switched_off_the_window_offers_nothing() {
        let mut app = crate::gui::screen_keys::tests::populated();
        let path = place("off");
        super::listen_at(path.clone());
        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new());
        assert!(!path.exists(), "the window opened a socket with the setting off");
        hand.app.set.claude_link = ClaudeLink::On;
        hand.frame(Vec::new());
        assert!(path.exists(), "the window did not open its socket once the setting was on");
        hand.app.set.claude_link = ClaudeLink::Off;
        hand.frame(Vec::new());
        assert!(!path.exists(), "the socket stayed after the setting was turned off");
    }

    /// WHAT IS THE WINDOW'S OWN IS LEFT TO THE PERSON: Claude is told so, and nothing changes.
    #[test]
    fn what_the_window_does_itself_is_refused() {
        let mut app = crate::gui::screen_keys::tests::populated();
        app.set.claude_link = ClaudeLink::On;
        let path = place("own");
        super::listen_at(path.clone());
        let steps = app.disk.edits.undo.len();
        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new());
        for tool in ["new_project", "undo"] {
            let job = claude(&path, tool, json!({}));
            for _ in 0..200 {
                if job.is_finished() {
                    break;
                }
                hand.frame(Vec::new());
                std::thread::sleep(Duration::from_millis(10));
            }
            let answer = job.join().expect("Claude's thread ends");
            assert_eq!(reply_of(&answer)["error"]["code"], "window-own", "{tool}: {answer}");
        }
        drop(hand);
        assert_eq!(app.disk.edits.undo.len(), steps, "a refused call left a step");
    }

    /// Run whole frames until Claude's thread is done; its answer.
    pub(crate) fn served<T>(hand: &mut Hand, job: std::thread::JoinHandle<T>) -> T {
        for _ in 0..1000 {
            if job.is_finished() {
                break;
            }
            hand.frame(Vec::new());
            std::thread::sleep(Duration::from_millis(10));
        }
        job.join().expect("Claude's thread ends")
    }

    /// CLAUDE, on a thread of its own: a run of calls on one link, and their answers.
    pub(crate) fn claude_says(path: &std::path::Path, calls: Vec<(&'static str, Value)>) -> std::thread::JoinHandle<Vec<Value>> {
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let mut link = Link::connect(&path, Duration::from_secs(60)).unwrap_or_else(|e| panic!("Claude did not reach the window: {e:?}"));
            calls.into_iter().map(|(tool, args)| link.call(tool, args).unwrap_or_else(|e| panic!("the window did not answer {tool}: {e:?}"))).collect()
        })
    }

    /// STEP INTO THE PART OF `body`, LOOK AT `at` AND CLICK IT, with nothing selected before: the first frame fits the
    /// camera to the scene, so the aim comes after it.
    pub(crate) fn aim_and_click(hand: &mut Hand, body: Id, at: [f64; 3], scale: f32) {
        hand.frame(Vec::new());
        hand.key(egui::Key::Escape);
        // in an assembly a click takes the whole part; a face is clicked inside its part, as a person steps in first
        if let Some(part) = hand.app.project.body_owner(body) {
            hand.app.enter_component(part);
        }
        hand.look_at(at, scale);
        hand.frame(Vec::new());
        hand.click(at);
    }

    /// A POINT ON A FACE THE ROUNDING MADE, in the world: the middle of one of its triangles, which lies on the surface,
    /// where the middle of the whole curved face would lie inside the body.
    pub(crate) fn on_the_rounding(app: &App, fillet: Id, body: Id) -> [f64; 3] {
        let b = app.project.bodies.iter().find(|b| b.id == body).expect("the rounded body");
        let face = b.faces.iter().find(|f| app.project.names.get(f.id).is_some_and(|n| n.feature == fillet)).expect("a face the rounding made");
        let tri = b.mesh.triangle(face.triangles[face.triangles.len() / 2] as usize);
        let mid = [(tri[0].x + tri[1].x + tri[2].x) / 3.0, (tri[0].y + tri[1].y + tri[2].y) / 3.0, (tri[0].z + tri[1].z + tri[2].z) / 3.0];
        let wt = app.project.body_display_transform(body, qymcad_ui_state::current_ctx_id(&app.active_path, &app.project));
        qymcad_core::feature::apply12(&wt, mid)
    }

    /// "MAKE THIS ROUNDING 0.2 MM SMALLER" (US-10). The person clicks the rounding in the window; Claude reads the
    /// selection, finds the feature that made it, and gives that feature a radius 0.2 smaller - one step of undo.
    #[test]
    fn a_selected_rounding_gets_smaller() {
        let mut app = crate::gui::screen_keys::tests::populated();
        app.set.claude_link = ClaudeLink::On;
        let path = place("round");
        super::listen_at(path.clone());
        let node = app.project.timeline.iter().find(|n| qymcad_doc::report::kind_of(&n.kind) == "Fillet").expect("the fixture's rounding");
        let (fillet, body) = (node.id, node.kind.body().expect("the rounding makes a body"));
        // THE RADIUS AS BUILT: the round surface of the rounding in the live body, not a number stored beside it
        let radius = |app: &App| {
            let shape = app.live.shapes.get(&body).expect("the rounded body is live");
            let faces = app.project.regen_faces.get(&body).expect("the rounded body has faces");
            faces.iter().filter(|f| app.project.names.get(f.id).is_some_and(|n| n.feature == fillet)).find_map(|f| shape.face_cylinder(f.id).map(|(_, _, r)| r)).expect("a round face of the rounding")
        };
        let before = radius(&app);
        let at = on_the_rounding(&app, fillet, body);

        let mut hand = Hand::new(&mut app);
        aim_and_click(&mut hand, body, at, 60.0);
        let taken = hand.app.chosen.sel;
        assert!(matches!(taken, qymcad_ui_state::Sel::Face(..) | qymcad_ui_state::Sel::Edge(..)), "setup: the click on the rounding took neither a face nor an edge");

        let read = served(&mut hand, claude_says(&path, vec![("get_selection", json!({}))]));
        let item = &reply_of(&read[0])["selected"][0];
        assert_eq!(item["made_by"]["feature"], json!(fillet), "the selection does not name the rounding as its author: {}", read[0]);
        assert_eq!(item["made_by"]["kind"], "Fillet", "{item}");
        let said = item["made_by"]["sizes"].as_array().and_then(|s| s.iter().find(|s| s["key"] == "radius")).and_then(|s| s["value"].as_f64()).expect("the rounding's radius");
        assert!((said - before).abs() < 1e-6, "the selection told radius {said}, the rounding is built at {before}");

        let steps = hand.app.disk.edits.undo.len();
        let done = served(&mut hand, claude_says(&path, vec![("edit_feature", json!({ "feature": fillet, "values": { "radius": said - 0.2 } }))]));
        assert_eq!(reply_of(&done[0])["ok"], json!(true), "{}", done[0]);
        assert_eq!(hand.app.disk.edits.undo.len(), steps + 1, "reading the selection and changing the rounding are not one step");
        drop(hand);
        assert!((radius(&app) - (before - 0.2)).abs() < 1e-6, "the rounding is built at {} after the change, not {}", radius(&app), before - 0.2);
    }

    /// A ROUNDING THAT FOLLOWS A PARAMETER keeps following it: the change is written into the rounding's own expression
    /// ("r - 0.2"), and the parameter, which other features may read, stays as it was.
    #[test]
    fn a_selected_rounding_that_follows_a_parameter_keeps_following_it() {
        let mut app = crate::gui::screen_keys::tests::populated();
        app.set.claude_link = ClaudeLink::On;
        let path = place("round-expr");
        super::listen_at(path.clone());
        let node = app.project.timeline.iter().find(|n| qymcad_doc::report::kind_of(&n.kind) == "Fillet").expect("the fixture's rounding");
        let (fillet, body) = (node.id, node.kind.body().expect("the rounding makes a body"));
        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new());
        let set = served(&mut hand, claude_says(&path, vec![("set_parameter", json!({ "name": "r", "expr": "1" })), ("edit_feature", json!({ "feature": fillet, "values": { "radius": "r" } }))]));
        assert!(set.iter().all(|a| reply_of(a)["ok"] == json!(true)), "setup: {set:?}");
        drop(hand);

        let at = on_the_rounding(&app, fillet, body);
        let mut hand = Hand::new(&mut app);
        aim_and_click(&mut hand, body, at, 60.0);
        let read = served(&mut hand, claude_says(&path, vec![("get_selection", json!({}))]));
        let radius = reply_of(&read[0])["selected"][0]["made_by"]["sizes"].as_array().and_then(|s| s.iter().find(|s| s["key"] == "radius").cloned()).expect("the rounding's radius");
        assert_eq!(radius["expr"], "r", "the selection does not say the radius follows r: {radius}");
        let done = served(&mut hand, claude_says(&path, vec![("edit_feature", json!({ "feature": fillet, "values": { "radius": "r - 0.2" } }))]));
        assert_eq!(reply_of(&done[0])["ok"], json!(true), "{}", done[0]);
        drop(hand);
        assert_eq!(app.project.feat_dims.get(&fillet).and_then(|m| m.get("radius")).map(String::as_str), Some("r - 0.2"), "the expression was not kept");
        assert!((app.project.parameters.iter().find(|p| p.name == "r").map(|p| p.value).expect("the parameter") - 1.0).abs() < 1e-12, "the parameter changed");
    }

    /// WHAT IS SELECTED IS TOLD BY ITS KIND, with the key the tools take: a face and an edge clicked in the scene, and
    /// nothing at all. Every kind of selection the window has is turned into one (`picked` matches them all, so a new
    /// kind does not compile without its line).
    #[test]
    fn what_is_selected_is_told_by_its_kind() {
        let mut app = crate::gui::screen_keys::tests::populated();
        app.set.claude_link = ClaudeLink::On;
        let path = place("kinds");
        super::listen_at(path.clone());
        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new());
        hand.key(egui::Key::Escape); // the fixture comes with a sketch selected; Escape lets it go, as by hand
        let nothing = served(&mut hand, claude_says(&path, vec![("get_selection", json!({}))]));
        assert_eq!(reply_of(&nothing[0])["selected"], json!([]), "{}", nothing[0]);
        assert!(reply_of(&nothing[0])["hint"].is_string(), "an empty selection gives no hint: {}", nothing[0]);
        drop(hand);

        // the window says the language the person reads it in; the answer itself stays English
        app.set.language = "uk".into();
        crate::gui::apply_language(&app.set);
        let mut hand = Hand::new(&mut app);
        let told = served(&mut hand, claude_says(&path, vec![("get_selection", json!({}))]));
        assert_eq!(reply_of(&told[0])["language"], json!("uk"), "the window did not say its language: {}", told[0]);
        assert!(reply_of(&told[0])["hint"].as_str().is_some_and(|h| h.starts_with("Nothing is selected")), "the answer left English: {}", told[0]);
        assert_eq!(qymcad_i18n::language(), "uk", "the call left the window in another language");
        drop(hand);

        // the top face of the plate, clicked in its middle
        let node = app.project.timeline.iter().find(|n| qymcad_doc::report::kind_of(&n.kind).starts_with("Extrude")).expect("the plate");
        let body = node.kind.body().expect("the plate's body");
        let wt = app.project.body_display_transform(body, qymcad_ui_state::current_ctx_id(&app.active_path, &app.project));
        let top = app.project.regen_faces.get(&body).and_then(|fs| fs.iter().max_by(|a, b| a.area.total_cmp(&b.area).then(a.centroid.z.total_cmp(&b.centroid.z)))).cloned().expect("a face");
        let at = qymcad_core::feature::apply12(&wt, [top.centroid.x, top.centroid.y, top.centroid.z]);
        // at the top of the assembly a click takes the whole part, told by the name a person reads
        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new());
        hand.key(egui::Key::Escape);
        hand.look_at(at, 8.0);
        hand.frame(Vec::new());
        hand.click(at);
        let read = served(&mut hand, claude_says(&path, vec![("get_selection", json!({}))]));
        let item = &reply_of(&read[0])["selected"][0];
        assert_eq!(item["what"], "part", "{}", read[0]);
        let name = item["name"].as_str().unwrap_or_default();
        assert!(!name.is_empty() && !name.contains('#') && !name.starts_with("name-"), "the part is told by the document's stored key, not its name: {name:?}");
        assert_eq!(item["use"]["part"], item["key"], "{item}");
        drop(hand);

        let mut hand = Hand::new(&mut app);
        aim_and_click(&mut hand, body, at, 8.0);
        let read = served(&mut hand, claude_says(&path, vec![("get_selection", json!({}))]));
        let item = &reply_of(&read[0])["selected"][0];
        assert_eq!(item["what"], "face", "{}", read[0]);
        assert_eq!(item["use"]["face"]["ids"][0], item["key"], "the face is not handed on by its own key: {item}");
        assert_eq!(item["use"]["body"]["body"], item["body"], "{item}");
        assert!(item["normal"].is_array() && item["kind"].is_string(), "the face is not told by its kind and place: {item}");
    }

    /// THE PICTURE IN THE WINDOW IS WHAT THE PERSON SEES: drawn through their camera, not fitted to the model. Aimed
    /// at the plate, the plate covers much of it; turned away to empty space, nothing is in sight - and the answer
    /// says so. A side named explicitly is still drawn from that side, fitted; and `render: true` on a call shows the
    /// person's view too.
    #[test]
    fn the_picture_is_what_the_person_sees() {
        let mut app = crate::gui::screen_keys::tests::populated();
        app.set.claude_link = ClaudeLink::On;
        let path = place("look");
        super::listen_at(path.clone());
        let node = app.project.timeline.iter().find(|n| qymcad_doc::report::kind_of(&n.kind).starts_with("Extrude")).expect("the plate");
        let body = node.kind.body().expect("the plate's body");
        let wt = app.project.body_display_transform(body, qymcad_ui_state::current_ctx_id(&app.active_path, &app.project));
        let middle = app
            .project
            .regen_faces
            .get(&body)
            .and_then(|fs| fs.iter().max_by(|a, b| a.area.total_cmp(&b.area)))
            .map(|f| qymcad_core::feature::apply12(&wt, [f.centroid.x, f.centroid.y, f.centroid.z]))
            .expect("a face");

        let mut hand = Hand::new(&mut app);
        hand.frame(Vec::new());
        hand.look_at(middle, 30.0);
        hand.frame(Vec::new());
        let aimed = served(&mut hand, claude_says(&path, vec![("render", json!({}))]));
        let aimed = reply_of(&aimed[0]);
        assert_eq!(aimed["view"], "window", "the picture in the window is not drawn through the person's eye: {aimed}");
        let near = aimed["filled"].as_f64().expect("the share the model covers");
        assert!(near > 0.2, "the camera aimed at the plate shows it covering only {near}");

        hand.look_at([middle[0] + 10_000.0, middle[1], middle[2]], 4.0);
        hand.frame(Vec::new());
        let away = served(&mut hand, claude_says(&path, vec![("render", json!({})), ("render", json!({ "view": "top" })), ("cylinder", json!({ "radius": 2, "height": 2, "render": true }))]));
        assert_eq!(reply_of(&away[0])["filled"].as_f64(), Some(0.0), "the camera turned to empty space still shows the model: {}", reply_of(&away[0]));
        let top = reply_of(&away[1]);
        assert!(top["view"] == "top" && top["filled"].as_f64().is_some_and(|f| f > 0.1), "a side named is not drawn from that side, fitted: {top}");
        assert_eq!(reply_of(&away[2])["picture"]["view"], "window", "render: true in the window does not show the person's view: {}", reply_of(&away[2]));
    }
}
