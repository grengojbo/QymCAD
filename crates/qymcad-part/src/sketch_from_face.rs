//! SKETCH FROM FACE: a flat face of a body is clicked, the boss or pocket standing on it is lit, and Enter lays a sketch
//! on the face with its outline copied in as curves of its own, fully defined and placed from the outline of the face
//! it stands on (`Project::add_sketch_from_face`). Replace in the bar also takes the boss or pocket off and lays it again
//! by the sketch, one node whose height is a field at the geometry (`Project::add_redrawn`): from then on the sizes of
//! the sketch drive the body. Made for a body come from a STEP file, which has no sketch behind its bosses.
use qymcad_core::feature::{FaceKey, FeatureKind};
use qymcad_core::model::{FaceSketch, FromFace, Id, PrismKind, RimBlend, RimCut};
use qymcad_ui_state::{CmdParam, PartCtx, RedrawInHand};

/// The code of the command.
pub const SKETCH_FROM_FACE: u8 = 39;

/// What Enter does, as the operation of the bar (`FeatTarget::op`) holds it.
pub const JUST_SKETCH: u8 = 0;
pub const REPLACE: u8 = 1;

/// How far the triangles of a face may turn from its normal (1 - cos) and the face still be flat.
const FLAT: f64 = 1e-6;

fn height_field(val: f64) -> CmdParam {
    CmdParam::new("f-redrawn-height", "height", val, 0.01, 100000.0)
}

/// TAKE THE TOOL: it waits for a click on a flat face, or takes at once the face selected before it.
pub fn start_sketch_from_face_cmd(pc: &mut PartCtx) {
    let before = *pc.sel;
    pc.cmd.open(pc.armed, SKETCH_FROM_FACE, *pc.mode_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, pc.view_restore);
    *pc.mode_3d = true;
    pc.gsel.edges.clear();
    pc.gsel.faces.clear();
    pc.gsel.faces_body = None;
    pc.recognise.redraw = None;
    pc.feat.op = REPLACE;
    pc.cmd.params = Vec::new();
    pc.cmd.focus = false;
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-sketch-from-face"));
    if let qymcad_ui_state::Sel::Face(mesh, face) = before {
        sketch_from_face_at(pc, mesh, Some(face));
    }
}

/// Whether a face of a body is flat: every triangle of it turned as the face is.
fn flat(pc: &PartCtx, at: qymcad_ui_state::made_by::FaceAt) -> bool {
    let b = &pc.project.bodies[at.mesh];
    let f = &b.faces[at.face];
    f.triangles.iter().all(|&t| {
        let [p, q, r] = b.mesh.triangle(t as usize);
        let (u, v) = ([q.x - p.x, q.y - p.y, q.z - p.z], [r.x - p.x, r.y - p.y, r.z - p.z]);
        let c = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
        len < 1e-12 || (c[0] * f.normal[0] + c[1] * f.normal[1] + c[2] * f.normal[2]) / len > 1.0 - FLAT
    })
}

/// A CLICK ON FACE `face` OF BODY `mesh` with the tool in hand: a flat face is taken, and what stands on it found - a
/// boss or a pocket to redraw, or the words why there is none, Sketch only being left then. The face clicked again is
/// let go.
pub fn sketch_from_face_at(pc: &mut PartCtx, mesh: usize, face: Option<usize>) {
    let (Some(body), Some(fi)) = (pc.project.mesh_id(mesh), face) else { return };
    let Some(f) = pc.project.bodies.get(mesh).and_then(|b| b.faces.get(fi)) else { return };
    let key = FaceKey { index: 0, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id };
    if pc.recognise.redraw.as_ref().is_some_and(|h| h.body == body && h.face.id == key.id) {
        pc.recognise.redraw = None;
        *pc.sel = qymcad_ui_state::Sel::None;
        refresh(pc);
        return;
    }
    if !flat(pc, qymcad_ui_state::made_by::FaceAt { mesh, face: fi }) {
        *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("error-prism-face-not-flat"));
        return;
    }
    let found = qymcad_ui_state::with_kernel(&mut pc.rebuild(), |project, k| project.prism_at(body, key.id, k));
    let told = match &found {
        Ok(p) => {
            let what = if p.kind == PrismKind::Boss { "msg-sketch-from-face-boss" } else { "msg-sketch-from-face-pocket" };
            let mut told = qymcad_i18n::tr1(what, "h", &qymcad_i18n::num(p.height, 2));
            // a rim rounded or cut is named: it goes with the boss and comes back the same size
            let rims = [p.rims.at_face.map(|r| RimTold { rim: r, at: "msg-rim-at-face" }), p.rims.at_base.map(|r| RimTold { rim: r, at: "msg-rim-at-base" })];
            for r in rims.into_iter().flatten() {
                let how = if r.rim.cut == RimCut::Round { "msg-rim-round" } else { "msg-rim-chamfer" };
                told = format!("{told}, {} {}", qymcad_i18n::tr1(how, "d", &qymcad_i18n::num(r.rim.size, 2)), qymcad_i18n::tr(r.at));
            }
            told
        }
        Err(e) => qymcad_i18n::tr(&e.key()),
    };
    let prism = found.ok();
    pc.recognise.redraw = Some(RedrawInHand { body, face: key, prism, told });
    refresh(pc);
}

/// The faces lit, the field of the height and the status line as the face held and the operation of the bar stand.
fn refresh(pc: &mut PartCtx) {
    let Some(held) = pc.recognise.redraw.as_ref() else {
        pc.gsel.faces.clear();
        pc.gsel.faces_body = None;
        pc.cmd.params = Vec::new();
        pc.cmd.focus = false;
        *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-sketch-from-face"));
        return;
    };
    let replace = pc.feat.op == REPLACE && held.prism.is_some();
    // what Enter takes off is lit: the face, and with Replace the whole boss or pocket
    pc.gsel.faces = match held.prism.as_ref().filter(|_| replace) {
        Some(p) => p.faces().into_iter().collect(),
        None => std::iter::once(held.face.id).collect(),
    };
    pc.gsel.faces_body = Some(held.body);
    let enter = if replace { "msg-sketch-from-face-enter-replace" } else { "msg-sketch-from-face-enter-sketch" };
    *pc.status = qymcad_ui_state::Notice::info(format!("{}. {}", held.told, qymcad_i18n::tr(enter)));
    match held.prism.as_ref().filter(|_| replace) {
        Some(p) => {
            let h = pc.cmd.params.iter().find(|f| f.key == "height").map(|f| f.val).unwrap_or(p.height);
            pc.cmd.params = vec![height_field(h)];
            pc.cmd.focus = true;
        }
        None => {
            pc.cmd.params = Vec::new();
            pc.cmd.focus = false;
        }
    }
}

/// THE OPERATION OF THE BAR CHANGED: the faces lit, the field and the status line follow it.
pub fn sketch_from_face_mode_changed(pc: &mut PartCtx) {
    refresh(pc);
}

/// Whether Enter has what it needs: a flat face held.
pub fn sketch_from_face_ready(pc: &PartCtx) -> bool {
    pc.recognise.redraw.is_some()
}

/// A rim of the prism held, and where it runs, for the status line.
struct RimTold {
    rim: RimBlend,
    at: &'static str,
}

/// What Enter laid: the sketch, and the node that redrew the boss or pocket by it.
struct Laid {
    sketch: FaceSketch,
    node: Option<Id>,
}

/// APPLY: the sketch taken off the face, and with Replace the boss or pocket redrawn by it, its height the field's - a
/// number, or an expression kept on the node. Answers the body of the node, or the sketch for Sketch only; what was laid
/// is told in words for the status line after Enter (`RecogniseTool::laid`).
pub fn apply_sketch_from_face_cmd(pc: &mut PartCtx) -> Option<Id> {
    let held = pc.recognise.redraw.clone()?;
    let replace = pc.feat.op == REPLACE;
    let height = qymcad_ui_state::cmd_val(pc.cmd, "height");
    let outline = held.prism.as_ref().map(|p| p.outline.clone()).unwrap_or_default();
    let profile = held.prism.as_ref().map(|p| p.profile.clone()).unwrap_or_default();
    let name = qymcad_i18n::tr1("g-sketch-n", "n", &(pc.project.sketches.len() + 1).to_string());
    let prism = held.prism.clone().filter(|_| replace);
    let laid = qymcad_ui_state::with_kernel(&mut pc.rebuild(), |project, k| {
        let sketch = project.add_sketch_from_face(FromFace { body: held.body, face: held.face, profile: &profile, outline: &outline, name }, k)?;
        let node = prism.as_ref().map(|p| project.add_redrawn(held.body, p, sketch.sketch));
        Some(Laid { sketch, node })
    });
    let Some(Laid { sketch, node }) = laid else {
        *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-sketch-from-face-no-outline"));
        return None;
    };
    if let Some(node) = node {
        if let Some(FeatureKind::Redrawn { height: h, .. }) = pc.project.timeline.iter_mut().find(|n| n.id == node).map(|n| &mut n.kind) {
            *h = height;
        }
        super::store_cmd_exprs(pc.cmd, pc.project, node);
    }
    pc.recognise.laid = told_laid(&sketch);
    node.or(Some(sketch.sketch))
}

/// What a sketch taken off a face laid, in words: its relations and sizes, whether it is fully defined, and each size
/// rounded - "29.97 -> 30.0".
fn told_laid(s: &FaceSketch) -> String {
    let d = &s.defined;
    let state = if d.free == 0 { qymcad_i18n::tr("dof-fully-defined") } else { qymcad_i18n::tr1("sk-dof-n", "n", &d.free.to_string()) };
    let mut out = qymcad_i18n::trn("msg-sketch-from-face-laid", &[("rel", &d.relations.to_string()), ("sizes", &d.sizes.to_string()), ("state", &state)]);
    if !d.rounded.is_empty() {
        let list: Vec<String> = d.rounded.iter().map(|r| format!("{} -> {}", qymcad_i18n::num(r.was, 2), qymcad_i18n::num(r.now, 1))).collect();
        out = format!("{out}; {}", qymcad_i18n::tr1("msg-sketch-from-face-rounded", "list", &list.join(", ")));
    }
    out
}

/// A DOUBLE CLICK ON A BOSS OR POCKET REDRAWN reopens the tool on its height: its faces lit on the body it was taken off,
/// the height in the field - an expression as it was typed - and Enter changes the node rather than laying another. Its
/// outline is the sketch's: a double click on the sketch edits it.
pub fn open_redrawn(pc: &mut PartCtx, fid: Id, kind: &FeatureKind, was_3d: bool) {
    let FeatureKind::Redrawn { src, ref faces, height, .. } = *kind else { return };
    pc.cmd.open(pc.armed, SKETCH_FROM_FACE, was_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, &mut *pc.view_restore);
    *pc.mode_3d = true;
    pc.recognise.redraw = None;
    pc.feat.op = REPLACE;
    pc.gsel.faces = super::live_picks(&*pc.project, src, faces, true).into_iter().collect();
    pc.gsel.faces_body = Some(src);
    pc.cmd.params = vec![super::cmd_param_from(&*pc.project, fid, "f-redrawn-height", "height", height, 0.01, 100000.0)];
    pc.cmd.focus = true;
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-edit-redrawn"));
}

/// The height written into a boss or pocket redrawn being edited; a field the tool does not show leaves it as it was.
pub fn edit_redrawn(cmd: &qymcad_ui_state::FeatCommand, height: &mut f64) {
    if cmd.params.iter().any(|p| p.key == "height") {
        *height = qymcad_ui_state::cmd_val(cmd, "height");
    }
}
