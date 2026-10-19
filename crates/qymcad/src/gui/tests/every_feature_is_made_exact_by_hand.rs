//! EVERY HOLE, ROUNDING AND CHAMFER OF A BODY MADE EXACT BY HAND: a plate comes in from a STEP file - drilled 4 blind, 5
//! and 6 through, one corner rounded 4 and another cut back 3 - "Recognise all" is taken and the plate clicked: five
//! groups in the list beside it. The rounding's group unticked, the chamfer clicked in the view to leave it out, and
//! Enter: one node, the three holes made exact, the plate holding what it held. A double click on its row reopens the
//! list with the sizes. A block whose edges are rounded in a chain, come in as a printer's file and recognised, holds a
//! group the kernel cannot lay: the trial finds it, leaves it out with its reason, and the rest is laid.
use crate::gui::hand::Hand;
use crate::gui::App;
use qymcad_kernel::specimen::{self, Specimen};
use qymcad_kernel::Shape;
use qymcad_ui_state::Taking;

/// A file written for the check, and the folder holding it, removed when the check is done with it.
struct Written {
    _folder: crate::gui::check_folder::tests::CheckFolder,
    path: String,
}

fn placed(s: &Shape, by: [f64; 3]) -> Shape {
    s.moved(&[1.0, 0.0, 0.0, by[0], 0.0, 1.0, 0.0, by[1], 0.0, 0.0, 1.0, by[2]]).expect("placed")
}

fn block() -> Shape {
    specimen::extrude(&[0.0, 0.0, 60.0, 0.0, 60.0, 40.0, 0.0, 40.0], 10.0).expect("a block")
}

fn upright(s: &Shape, (x, y): (f64, f64)) -> u32 {
    s.edges_info().into_iter().find(|e| e.poly.iter().all(|p| (p[0] as f64 - x).abs() < 1e-6 && (p[1] as f64 - y).abs() < 1e-6)).map(|e| e.id).expect("the upright edge")
}

/// A drill of radius `r` from z = `from` up `h`, placed at `at`.
struct Drill {
    at: [f64; 2],
    r: f64,
    from: f64,
    h: f64,
}

fn drilled(s: Shape, d: Drill) -> Shape {
    s.combined(&placed(&specimen::cylinder(d.r, d.h).expect("a drill"), [d.at[0], d.at[1], d.from]), 0).expect("drilled")
}

/// The plate: 60 x 40 x 10 from the origin, drilled 4 blind at (45, 20), 5 through at (30, 10) and 6 through at
/// (15, 20), its corner at (60, 40) rounded 4 and its corner at the origin cut back 3.
pub(crate) fn plate() -> Shape {
    let p = block();
    let p = p.filleted(4.0, &[upright(&p, (60.0, 40.0))]).expect("rounded");
    let p = p.chamfered(3.0, &[upright(&p, (0.0, 0.0))]).expect("cut");
    let p = drilled(p, Drill { at: [15.0, 20.0], r: 3.0, from: -1.0, h: 12.0 });
    let p = drilled(p, Drill { at: [30.0, 10.0], r: 2.5, from: -1.0, h: 12.0 });
    drilled(p, Drill { at: [45.0, 20.0], r: 2.0, from: 5.0, h: 6.0 })
}

fn chained() -> Shape {
    let p = block();
    let ups: Vec<u32> = [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)].iter().map(|&at| upright(&p, at)).collect();
    let p = p.filleted(4.0, &ups).expect("rounded upright");
    let top: Vec<u32> = p.edges_info().into_iter().filter(|e| e.poly.iter().all(|q| (q[2] as f64 - 10.0).abs() < 1e-6)).map(|e| e.id).collect();
    let p = p.filleted(2.0, &top).expect("rounded round the top");
    drilled(p, Drill { at: [15.0, 20.0], r: 3.0, from: -1.0, h: 12.0 })
}

/// Frames run, as they do by themselves, until nothing is being built.
fn at_rest(h: &mut Hand) {
    for _ in 0..600 {
        h.frame(Vec::new());
        h.app.drain_bg_for_test();
        if h.app.regen.busy.is_none() && h.app.regen.bg.is_empty() && !h.app.regen.wanted {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// The body brought in last, its part entered, as a person works on it.
fn worked_on(h: &mut Hand) -> qymcad_core::model::Id {
    let body = h.app.project.bodies.last().map(|b| b.id).expect("the body came in");
    if let Some(part) = h.app.project.body_owner(body) {
        h.app.enter_component(part);
    }
    body
}

/// A group the tool holds: in words, and whether it is taken.
#[derive(Debug)]
struct Row {
    told: String,
    taking: Taking,
}

fn groups(h: &Hand) -> Vec<Row> {
    h.app.params.recognise.all.as_ref().map(|all| all.groups.iter().map(|g| Row { told: qymcad_part::group_told(g), taking: g.taking.clone() }).collect()).unwrap_or_default()
}

#[test]
fn a_plate_clicked_gives_its_groups_and_enter_makes_the_ticked_exact_in_one_node() {
    let mut app = App::default();
    let folder = crate::gui::check_folder::tests::CheckFolder::new("recognise-all");
    let file = Written { path: folder.file("plate.step").to_string_lossy().into_owned(), _folder: folder };
    let exact = plate();
    qymcad_kernel::write_step_of(&exact, &file.path).expect("the STEP is written");
    let mut h = Hand::new(&mut app);
    h.import(&file.path);
    h.press_word(&crate::i18n::tr("import-scale-import"), egui::pos2(700.0, 450.0));
    at_rest(&mut h);
    worked_on(&mut h);
    h.look_at([30.0, 20.0, 5.0], 6.0).frame(Vec::new());
    h.tool(qymcad_part::RECOGNISE_ALL).click([40.0, 32.0, 10.0]).frame(Vec::new());
    let found = groups(&h);
    assert_eq!(found.len(), 5, "the plate clicked gives not its five groups: {found:?}; the program says {}", h.app.status);
    assert!(found.iter().all(|r| r.taking == Taking::In), "a group found is not taken: {found:?}");
    // the rounding's group unticked in the list beside the plate
    let round = found.iter().find(|r| r.told.contains("R4")).map(|r| r.told.clone()).expect("the group of the rounding");
    assert!(h.press_tick_before(&round, egui::pos2(700.0, 400.0)), "the row {round:?} has no tick on screen");
    h.frame(Vec::new());
    // the chamfer clicked in the view, turned to face it: left out
    h.orbit(-2.27, 0.6).frame(Vec::new());
    h.click([1.5, 1.5, 5.0]).frame(Vec::new());
    let now = groups(&h);
    let taken: Vec<&String> = now.iter().filter(|r| r.taking == Taking::In).map(|r| &r.told).collect();
    assert_eq!(taken.len(), 3, "after the rounding unticked and the chamfer clicked, not the three holes are taken: {now:?}");
    let nodes = h.app.project.timeline.len();
    h.enter();
    at_rest(&mut h);
    assert_eq!(h.app.project.timeline.len(), nodes + 1, "Enter laid not one node: {}", h.app.status);
    let node = h.app.project.timeline.last().expect("the node");
    let qymcad_core::feature::FeatureKind::Remade { ref items, body, .. } = node.kind else { panic!("the last node is not the features made exact: {:?}", node.kind) };
    assert_eq!(items.len(), 3, "the node holds not the three holes");
    assert!(items.iter().all(|it| matches!(it.exact, qymcad_core::model::Exact::Hole { .. })), "the node holds what was left out");
    let built = h.app.live.shapes.get(&body).map(|s| s.volume());
    let said = h.app.project.regen_errors.get(&node.id).map(|e| format!("{e:?}"));
    assert!(built.is_some_and(|v| (v - exact.volume()).abs() < 1e-3 * exact.volume()), "the plate holds {built:?} mm^3, not {}; the node says {said:?}", exact.volume());
    // the row of the node reopens the list with its sizes
    let ti = h.app.project.timeline.len() - 1;
    let row = crate::gui::panels_tree::feature_row_label(&h.app.project, ti);
    h.key(egui::Key::Escape).frame(Vec::new());
    assert!(h.double_click_word(&row), "the row {row:?} is not in the tree");
    assert_eq!(h.app.tools.armed.cmd_kind(), qymcad_part::RECOGNISE_ALL, "the double click did not reopen the list");
    let keys: Vec<&str> = h.app.tools.cmd.params.iter().map(|p| p.key.as_str()).collect();
    assert_eq!(keys, ["diameter", "depth", "diameter.2", "diameter.3"], "the sizes of the three groups are not in the fields");
    assert_eq!(groups(&h).len(), 3, "the list reopened holds not the node's three groups");
}

/// A printer's file of `s`, written as a person's printer writes it.
fn printed(s: &Shape, name: &str) -> Written {
    let folder = crate::gui::check_folder::tests::CheckFolder::new(name);
    let path = folder.file("printed.3mf").to_string_lossy().into_owned();
    qymcad_io::export_3mf(&[specimen::mesh_merged(s, 0.01).expect("a mesh").mesh], &path).expect("the printer's file is written");
    Written { _folder: folder, path }
}

#[test]
fn a_group_that_does_not_lay_is_left_out_with_its_reason_and_the_rest_is_laid() {
    let mut app = App::default();
    let file = printed(&chained(), "recognise-all-chain");
    let mut h = Hand::new(&mut app);
    h.import(&file.path);
    at_rest(&mut h);
    worked_on(&mut h);
    h.look_at([30.0, 20.0, 5.0], 6.0).frame(Vec::new());
    // recognised into exact surfaces, as the tool does by itself
    h.tool(35).frame(Vec::new());
    h.click([30.0, 20.0, 10.0]).frame(Vec::new());
    h.enter();
    at_rest(&mut h);
    h.tool(qymcad_part::RECOGNISE_ALL).click([30.0, 20.0, 10.0]).frame(Vec::new());
    assert!(groups(&h).len() >= 2, "the recognised block gives no groups: {:?}; the program says {}", groups(&h), h.app.status);
    // the trial answers, and the popup reads its answer as it is drawn: a group refused is left out, and the rest is
    // tried again, until it lays
    let mut said = Vec::new();
    for _ in 0..8 {
        let Some(words) = h.trial_says() else { break };
        said.push(words);
        h.frame(Vec::new());
    }
    let out: Vec<Row> = groups(&h).into_iter().filter(|r| matches!(r.taking, Taking::Refused(_))).collect();
    assert!(!out.is_empty() && out.len() < groups(&h).len(), "the trial said {said:?}, and the groups are {:?}", groups(&h));
    assert!(h.trial_says().is_none(), "with {} groups left out the rest is still refused; the groups are {:?}", out.len(), groups(&h));
    let nodes = h.app.project.timeline.len();
    h.enter();
    at_rest(&mut h);
    assert_eq!(h.app.project.timeline.len(), nodes + 1, "Enter laid not one node: {}", h.app.status);
    let node = h.app.project.timeline.last().expect("the node");
    assert!(!h.app.project.regen_errors.contains_key(&node.id), "the node laid went red: {:?}", h.app.project.regen_errors.get(&node.id));
}
