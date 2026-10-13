//! SIZE ON THE BODY: two flat faces of a body standing parallel are clicked, the distance between them stands in a field
//! at the geometry, the face to move is chosen in the bar - the first, the second, or both by half - and Enter makes
//! them that far apart (`Project::add_face_gap`): one node whose number drives the body. Made for a body come from a STEP
//! or a mesh, which has no sketch to change a size in.
use qymcad_core::feature::{FaceKey, FeatureKind, GapMoves};
use qymcad_core::model::{Gap, Id};
use qymcad_ui_state::{CmdParam, PartCtx};

/// The code of the command.
pub const FACE_GAP: u8 = 38;

/// The face a size moves, as the operation of the bar (`FeatTarget::op`) holds it.
pub const MOVE_ONE: u8 = 0;
pub const MOVE_OTHER: u8 = 1;
pub const MOVE_BOTH: u8 = 2;

/// How far two flat faces may turn apart (1 - cos) and still stand parallel, and how near their planes may come and
/// still stand apart (mm) - the bounds the core holds a size to.
const TURN: f64 = 1e-6;
const NEAR: f64 = 1e-3;

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
    pc.cmd.open(pc.armed, FACE_GAP, *pc.mode_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, pc.view_restore);
    *pc.mode_3d = true;
    pc.gsel.edges.clear();
    pc.gsel.faces.clear();
    pc.gsel.faces_body = None;
    pc.recognise.gap.clear();
    pc.feat.op = MOVE_ONE;
    pc.cmd.params = Vec::new();
    // no field takes the keys until two faces are picked: the tool waits for clicks
    pc.cmd.focus = false;
    *pc.status = qymcad_i18n::tr("msg-face-gap");
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
/// parallel to it - and a third click puts a new second face in place of the old one. A face clicked again is let go.
pub fn face_gap_at(pc: &mut PartCtx, mesh: usize, face: Option<usize>) {
    let (Some(body), Some(fi)) = (pc.project.mesh_id(mesh), face) else { return };
    let Some(id) = pc.project.bodies.get(mesh).and_then(|b| b.faces.get(fi)).map(|f| f.id) else { return };
    if pc.gsel.faces_body != Some(body) {
        pc.recognise.gap.clear();
        pc.gsel.faces_body = Some(body);
    }
    if let Some(at) = pc.recognise.gap.iter().position(|&g| g == id) {
        // A FACE CLICKED AGAIN IS LET GO as any pick is: the face the click took is no longer held either
        pc.recognise.gap.remove(at);
        *pc.sel = qymcad_ui_state::Sel::None;
    } else {
        if pc.recognise.gap.len() == 2 {
            pc.recognise.gap.pop();
        }
        let first = pc.recognise.gap.first().and_then(|&a| key_of(pc, body, a));
        if let (Some(one), Some(other)) = (first, key_of(pc, body, id)) {
            if apart(&one, &other).is_none() {
                *pc.status = qymcad_i18n::tr("msg-gap-not-parallel");
                return;
            }
        }
        pc.recognise.gap.push(id);
    }
    refresh(pc);
}

/// The picks lit, the field of the size and the status line as the picks stand.
fn refresh(pc: &mut PartCtx) {
    pc.gsel.faces = pc.recognise.gap.iter().copied().collect();
    if pc.recognise.gap.is_empty() {
        pc.gsel.faces_body = None;
    }
    match (picked(pc), pc.recognise.gap.len()) {
        (Some(p), _) => {
            let d = apart(&p.one, &p.other).unwrap_or_default();
            pc.cmd.params = vec![size_field(d)];
            pc.cmd.focus = true;
            *pc.status = qymcad_i18n::tr1("msg-gap-picked", "d", &qymcad_i18n::num(d, 2));
        }
        (None, 1) => {
            pc.cmd.params = Vec::new();
            *pc.status = qymcad_i18n::tr("msg-face-gap-second");
        }
        _ => {
            pc.cmd.params = Vec::new();
            *pc.status = qymcad_i18n::tr("msg-face-gap");
        }
    }
}

/// Whether Enter has what it needs: two faces.
pub fn face_gap_ready(pc: &PartCtx) -> bool {
    pc.recognise.gap.len() == 2
}

/// APPLY: the two faces made the size of the field apart, by moving the face the bar names - a number, or an expression
/// kept on the node. A size typed as it stands changes nothing and lays nothing.
pub fn apply_face_gap_cmd(pc: &mut PartCtx) -> Option<Id> {
    let Some(p) = picked(pc) else {
        *pc.status = qymcad_i18n::tr("msg-face-gap");
        return None;
    };
    let size = qymcad_ui_state::cmd_val(pc.cmd, "gap");
    let typed = pc.cmd.params.iter().any(|f| f.key == "gap" && f.txt.trim().parse::<f64>().is_err());
    if !typed && apart(&p.one, &p.other).is_some_and(|d| (d - size).abs() < 1e-9) {
        *pc.status = qymcad_i18n::tr("msg-gap-unchanged");
        return None;
    }
    let node = pc.project.add_face_gap(p.body, Gap { one: p.one, other: p.other, size, moves: gap_moves(pc.feat.op) });
    super::store_cmd_exprs(pc.cmd, pc.project, node);
    Some(node)
}

/// A DOUBLE CLICK ON A SIZE reopens the tool on it: its two faces lit on the body they stand on, the face it moves in
/// the bar, its size in the field - an expression as it was typed - and Enter changes the node rather than laying another.
pub fn open_face_gap(pc: &mut PartCtx, fid: Id, kind: &FeatureKind, was_3d: bool) {
    let FeatureKind::FaceGap { src, ref one, ref other, gap, moves, .. } = *kind else { return };
    pc.cmd.open(pc.armed, FACE_GAP, was_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, &mut *pc.view_restore);
    *pc.mode_3d = true;
    let first = |r| super::live_picks(&*pc.project, src, r, true).into_iter().next();
    pc.recognise.gap = [first(one), first(other)].into_iter().flatten().collect();
    pc.gsel.faces = pc.recognise.gap.iter().copied().collect();
    pc.gsel.faces_body = Some(src);
    pc.feat.op = op_of(moves);
    pc.cmd.params = vec![super::cmd_param_from(&*pc.project, fid, "f-gap", "gap", gap, 0.01, 100000.0)];
    *pc.status = qymcad_i18n::tr("msg-edit-face-gap");
}

/// The size and the face it moves written into a size being edited.
pub fn edit_face_gap(cmd: &qymcad_ui_state::FeatCommand, op: u8, gap: &mut f64, moves: &mut GapMoves) {
    if cmd.params.iter().any(|p| p.key == "gap") {
        *gap = qymcad_ui_state::cmd_val(cmd, "gap");
    }
    *moves = gap_moves(op);
}
