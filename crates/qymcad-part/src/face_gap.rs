//! SIZE ON THE BODY: two flat faces of a body standing parallel are clicked, the distance between them stands in a field
//! at the geometry, the face to move is chosen in the bar - the first, the second, or both by half - and Enter makes
//! them that far apart (`Project::add_face_gap`): one node whose number drives the body. The wall of a hole clicked with
//! a flat face along its axis gives the distance from the axis to the face, and the hole moves
//! (`Project::add_hole_moved`). Made for a body come from a STEP or a mesh, which has no sketch to change a size in.
use qymcad_core::feature::{FaceKey, FeatureKind, GapMoves};
use qymcad_core::model::{Exact, Gap, HoleMove, Id};
use qymcad_meshfit::feature::{feature_at, Found, Pick};
use qymcad_ui_state::{CmdParam, PartCtx};

/// The face a size moves, as the operation of the bar (`FeatTarget::op`) holds it.
pub const MOVE_ONE: u8 = 0;
pub const MOVE_OTHER: u8 = 1;
pub const MOVE_BOTH: u8 = 2;

/// How far two flat faces may turn apart (1 - cos) and still stand parallel, and how near their planes may come and
/// still stand apart (mm) - the bounds the core holds a size to.
const TURN: f64 = 1e-6;
const NEAR: f64 = 1e-3;

/// How far the axis of a hole may lean to a face (the cosine between the face's normal and the hole's entry normal) and
/// still stand parallel to it - the bound the core holds.
const AXIS_TURN: f64 = 0.01;

/// How far the triangles of a face may turn from its normal (1 - cos) and the face still be flat.
const FLAT: f64 = 1e-6;

/// The face a size moves, read off the operation of the bar.
pub fn gap_moves(op: u8) -> GapMoves {
    match op {
        MOVE_OTHER => GapMoves::Other,
        MOVE_BOTH => GapMoves::Both,
        _ => GapMoves::One,
    }
}

fn op_of(moves: GapMoves) -> u8 {
    match moves {
        GapMoves::One => MOVE_ONE,
        GapMoves::Other => MOVE_OTHER,
        GapMoves::Both => MOVE_BOTH,
    }
}

fn size_field(val: f64) -> CmdParam {
    CmdParam::new("f-gap", "gap", val, 0.01, 100000.0)
}

pub fn start_face_gap_cmd(pc: &mut PartCtx) {
    pc.cmd.open(pc.armed, qymcad_ui_state::PartTool::FaceGap, *pc.mode_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, pc.view_restore);
    *pc.mode_3d = true;
    pc.gsel.edges.clear();
    pc.gsel.faces.clear();
    pc.gsel.faces_body = None;
    pc.recognise.gap.clear();
    pc.recognise.feature = None;
    pc.feat.op = MOVE_ONE;
    pc.cmd.params = Vec::new();
    // no field takes the keys until two faces are picked: the tool waits for clicks
    pc.cmd.focus = false;
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-face-gap"));
}

/// The key of face `id` of body `body`, as the body stands.
fn key_of(pc: &PartCtx, body: Id, id: u32) -> Option<FaceKey> {
    let b = pc.project.mesh_index(body).and_then(|mi| pc.project.bodies.get(mi))?;
    let f = b.faces.iter().find(|f| f.id == id)?;
    Some(FaceKey { index: 0, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id })
}

/// How far apart two flat faces stand, when they stand parallel and apart.
pub fn apart(one: &FaceKey, other: &FaceKey) -> Option<f64> {
    let n = one.normal;
    let turn = n[0] * other.normal[0] + n[1] * other.normal[1] + n[2] * other.normal[2];
    let d = ((0..3).map(|i| n[i] * (other.centroid[i] - one.centroid[i])).sum::<f64>()).abs();
    (turn.abs() > 1.0 - TURN && d > NEAR).then_some(d)
}

/// The two faces picked, in the order they were clicked, with the body they stand on.
fn picked(pc: &PartCtx) -> Option<Picked> {
    let body = pc.gsel.faces_body?;
    let (&a, &b) = (pc.recognise.gap.first()?, pc.recognise.gap.get(1)?);
    Some(Picked { body, one: key_of(pc, body, a)?, other: key_of(pc, body, b)? })
}

/// Two faces of a body picked for a size.
struct Picked {
    body: Id,
    one: FaceKey,
    other: FaceKey,
}

/// A CLICK ON FACE `face` OF BODY `mesh` with the tool in hand: the first face, then the second - which must stand
/// parallel to it - and a third click puts a new second face in place of the old one. A round wall clicked takes the hole
/// it belongs to; the flat face picked with it must run along its axis. A pick clicked again is let go.
pub fn face_gap_at(pc: &mut PartCtx, mesh: usize, face: Option<usize>) {
    let (Some(body), Some(fi)) = (pc.project.mesh_id(mesh), face) else { return };
    let Some(f) = pc.project.bodies.get(mesh).and_then(|b| b.faces.get(fi)) else { return };
    let id = f.id;
    if pc.gsel.faces_body != Some(body) {
        pc.recognise.gap.clear();
        pc.recognise.feature = None;
        pc.gsel.faces_body = Some(body);
    }
    if pc.recognise.feature.as_ref().is_some_and(|h| h.remake.faces.iter().any(|k| k.id == id)) {
        // THE HOLE CLICKED AGAIN IS LET GO, as any pick is
        pc.recognise.feature = None;
        *pc.sel = qymcad_ui_state::Sel::None;
    } else if let Some(at) = pc.recognise.gap.iter().position(|&g| g == id) {
        // A FACE CLICKED AGAIN IS LET GO as any pick is: the face the click took is no longer held either
        pc.recognise.gap.remove(at);
        *pc.sel = qymcad_ui_state::Sel::None;
    } else if !flat(pc, mesh, fi) {
        let b = &pc.project.bodies[mesh];
        let found = feature_at(&b.mesh, &b.faces, Pick { face: id, at: [f.centroid.x, f.centroid.y, f.centroid.z] }, 1.0);
        let Some((Found::Hole(h), remake)) = found.ok().and_then(|found| found.remake(&b.faces).map(|r| (found, r))) else {
            *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-gap-not-flat"));
            return;
        };
        let told = qymcad_i18n::tr1("msg-gap-hole", "d", &qymcad_i18n::num(h.diameter, 2));
        pc.recognise.gap.truncate(1);
        if let Some(plane) = pc.recognise.gap.first().and_then(|&g| key_of(pc, body, g)) {
            if !along(&remake.exact, &plane) {
                *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-hole-gap-across"));
                return;
            }
        }
        pc.recognise.feature = Some(qymcad_ui_state::FeatureInHand { body, remake, told });
    } else {
        let held = pc.recognise.feature.as_ref().map(|h| h.remake.exact.clone());
        let limit = if held.is_some() { 1 } else { 2 };
        if pc.recognise.gap.len() == limit {
            pc.recognise.gap.pop();
        }
        let Some(other) = key_of(pc, body, id) else { return };
        if let Some(exact) = held {
            if !along(&exact, &other) {
                *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-hole-gap-across"));
                return;
            }
        } else if let Some(one) = pc.recognise.gap.first().and_then(|&a| key_of(pc, body, a)) {
            if apart(&one, &other).is_none() {
                *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-gap-not-parallel"));
                return;
            }
        }
        pc.recognise.gap.push(id);
    }
    refresh(pc);
}

/// Whether face `fi` of body `mesh` is flat: every triangle of it turned as the face is.
fn flat(pc: &PartCtx, mesh: usize, fi: usize) -> bool {
    let b = &pc.project.bodies[mesh];
    let f = &b.faces[fi];
    f.triangles.iter().all(|&t| {
        let [p, q, r] = b.mesh.triangle(t as usize);
        let (u, v) = ([q.x - p.x, q.y - p.y, q.z - p.z], [r.x - p.x, r.y - p.y, r.z - p.z]);
        let c = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
        len < 1e-12 || (c[0] * f.normal[0] + c[1] * f.normal[1] + c[2] * f.normal[2]) / len > 1.0 - FLAT
    })
}

/// Whether the axis of a hole - square into its entry face - runs along flat face `plane`.
fn along(exact: &Exact, plane: &FaceKey) -> bool {
    let Exact::Hole { entry, .. } = exact else { return false };
    (0..3).map(|i| entry.normal[i] * plane.normal[i]).sum::<f64>().abs() < AXIS_TURN
}

/// How far the axis of a hole stands from the plane of a flat face.
fn axis_off(exact: &Exact, plane: &FaceKey) -> Option<f64> {
    let Exact::Hole { at, .. } = exact else { return None };
    Some((0..3).map(|i| plane.normal[i] * (at[i] - plane.centroid[i])).sum::<f64>().abs())
}

/// A hole picked with the flat face its distance is measured to.
struct HolePicked {
    body: Id,
    hole: qymcad_core::model::Remake,
    face: FaceKey,
}

fn hole_picked(pc: &PartCtx) -> Option<HolePicked> {
    let held = pc.recognise.feature.as_ref()?;
    let face = pc.recognise.gap.first().and_then(|&g| key_of(pc, held.body, g))?;
    Some(HolePicked { body: held.body, hole: held.remake.clone(), face })
}

/// The picks lit, the field of the size and the status line as the picks stand.
fn refresh(pc: &mut PartCtx) {
    let hole_faces: Vec<u32> = pc.recognise.feature.as_ref().map(|h| h.remake.faces.iter().map(|k| k.id).collect()).unwrap_or_default();
    pc.gsel.faces = pc.recognise.gap.iter().copied().chain(hole_faces).collect();
    if pc.gsel.faces.is_empty() {
        pc.gsel.faces_body = None;
    }
    if let Some(h) = hole_picked(pc) {
        let d = axis_off(&h.hole.exact, &h.face).unwrap_or_default();
        pc.cmd.params = vec![size_field(d)];
        pc.cmd.focus = true;
        *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr1("msg-hole-gap-picked", "d", &qymcad_i18n::num(d, 2)));
        return;
    }
    if let Some(held) = pc.recognise.feature.as_ref() {
        pc.cmd.params = Vec::new();
        *pc.status = qymcad_ui_state::Notice::info(format!("{}. {}", held.told, qymcad_i18n::tr("msg-hole-gap-face")));
        return;
    }
    match (picked(pc), pc.recognise.gap.len()) {
        (Some(p), _) => {
            let d = apart(&p.one, &p.other).unwrap_or_default();
            pc.cmd.params = vec![size_field(d)];
            pc.cmd.focus = true;
            *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr1("msg-gap-picked", "d", &qymcad_i18n::num(d, 2)));
        }
        (None, 1) => {
            pc.cmd.params = Vec::new();
            *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-face-gap-second"));
        }
        _ => {
            pc.cmd.params = Vec::new();
            *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-face-gap"));
        }
    }
}

/// Whether Enter has what it needs: two faces, or a hole and a face.
pub fn face_gap_ready(pc: &PartCtx) -> bool {
    if pc.recognise.feature.is_some() {
        pc.recognise.gap.len() == 1
    } else {
        pc.recognise.gap.len() == 2
    }
}

/// Whether a hole is held: the size then moves it, and the bar has no face to choose.
pub fn face_gap_holds_hole(pc: &PartCtx) -> bool {
    pc.recognise.feature.is_some()
}

/// APPLY: the two faces made the size of the field apart, by moving the face the bar names - a number, or an expression
/// kept on the node. A size typed as it stands changes nothing and lays nothing.
pub fn apply_face_gap_cmd(pc: &mut PartCtx) -> Result<Id, qymcad_ui_state::NotApplied> {
    let size = qymcad_ui_state::cmd_val(pc.cmd, "gap");
    let typed = pc.cmd.params.iter().any(|f| f.key == "gap" && f.txt.trim().parse::<f64>().is_err());
    if let Some(h) = hole_picked(pc) {
        if !typed && axis_off(&h.hole.exact, &h.face).is_some_and(|d| (d - size).abs() < 1e-9) {
            return Err(super::refused("msg-gap-unchanged"));
        }
        let node = pc.project.add_hole_moved(h.body, HoleMove { hole: h.hole, face: h.face, size });
        if let Err(e) = super::store_cmd_exprs(pc.cmd, pc.project, node) {
            return Err(super::formula_refused(&e));
        }
        return Ok(node);
    }
    let Some(p) = picked(pc) else {
        return Err(super::refused("msg-face-gap"));
    };
    if !typed && apart(&p.one, &p.other).is_some_and(|d| (d - size).abs() < 1e-9) {
        return Err(super::refused("msg-gap-unchanged"));
    }
    let node = pc.project.add_face_gap(p.body, Gap { one: p.one, other: p.other, size, moves: gap_moves(pc.feat.op) });
    if let Err(e) = super::store_cmd_exprs(pc.cmd, pc.project, node) {
        return Err(super::formula_refused(&e));
    }
    Ok(node)
}

/// A DOUBLE CLICK ON A SIZE reopens the tool on it: its two faces lit on the body they stand on, the face it moves in
/// the bar, its size in the field - an expression as it was typed - and Enter changes the node rather than laying another.
pub fn open_face_gap(pc: &mut PartCtx, fid: Id, kind: &FeatureKind, was_3d: bool) {
    if let FeatureKind::HoleMoved { src, ref faces, ref face, gap, .. } = *kind {
        pc.cmd.open(pc.armed, qymcad_ui_state::PartTool::FaceGap, was_3d);
        qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, &mut *pc.view_restore);
        *pc.mode_3d = true;
        pc.recognise.feature = None;
        pc.recognise.gap = super::live_picks(&*pc.project, src, face, true).into_iter().take(1).collect();
        pc.gsel.faces = super::live_picks(&*pc.project, src, faces, true).into_iter().chain(pc.recognise.gap.iter().copied()).collect();
        pc.gsel.faces_body = Some(src);
        pc.cmd.params = vec![super::cmd_param_from(&*pc.project, fid, "f-gap", "gap", gap, 0.01, 100000.0)];
        *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-edit-face-gap"));
        return;
    }
    let FeatureKind::FaceGap { src, ref one, ref other, gap, moves, .. } = *kind else { return };
    pc.cmd.open(pc.armed, qymcad_ui_state::PartTool::FaceGap, was_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, &mut *pc.view_restore);
    *pc.mode_3d = true;
    let first = |r| super::live_picks(&*pc.project, src, r, true).into_iter().next();
    pc.recognise.gap = [first(one), first(other)].into_iter().flatten().collect();
    pc.gsel.faces = pc.recognise.gap.iter().copied().collect();
    pc.gsel.faces_body = Some(src);
    pc.feat.op = op_of(moves);
    pc.cmd.params = vec![super::cmd_param_from(&*pc.project, fid, "f-gap", "gap", gap, 0.01, 100000.0)];
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-edit-face-gap"));
}

/// The size and the face it moves written into a size being edited.
pub fn edit_face_gap(cmd: &qymcad_ui_state::FeatCommand, op: u8, gap: &mut f64, moves: &mut GapMoves) {
    edit_gap(cmd, gap);
    *moves = gap_moves(op);
}

/// The size written into a size or a hole moved being edited; a field the tool does not show leaves it as it was.
pub fn edit_gap(cmd: &qymcad_ui_state::FeatCommand, gap: &mut f64) {
    if cmd.params.iter().any(|p| p.key == "gap") {
        *gap = qymcad_ui_state::cmd_val(cmd, "gap");
    }
}
