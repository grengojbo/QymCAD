//! SKETCH FROM FACE: a flat face of a body is clicked, the boss or pocket standing on it is lit, and Enter lays a sketch
//! on the face with its outline copied in as curves of its own, fully defined and placed from the outline of the face
//! it stands on (`Project::add_sketch_from_face`). Replace in the bar also takes the boss or pocket off and lays it again
//! by the sketch, one node whose height is a field at the geometry (`Project::add_redrawn`): from then on the sizes of
//! the sketch drive the body. Made for a body come from a STEP file, which has no sketch behind its bosses.
use qymcad_core::feature::{FaceKey, FeatureKind};
use qymcad_core::model::{FaceSketch, FromFace, Id, OutlineSizeKind, PrismKind, RimBlend, RimCut, Rims};
use qymcad_ui_state::{CmdParam, PartCtx, RedrawInHand};

/// What Enter does, as the operation of the bar (`FeatTarget::op`) holds it.
pub const JUST_SKETCH: u8 = 0;
pub const REPLACE: u8 = 1;

/// How far the triangles of a face may turn from its normal (1 - cos) and the face still be flat.
const FLAT: f64 = 1e-6;

fn height_field(val: f64) -> CmdParam {
    CmdParam::new("f-redrawn-height", "height", val, 0.01, 100000.0)
}

/// A RIM OF A BOSS OR A POCKET REDRAWN as a field beside its height: its key among the sizes of the node, and what it
/// is called by how it is made. Reported behaviour: a rim rounded R2 was named only in the status line before Enter,
/// and could not be changed.
struct RimField {
    key: &'static str,
    round: &'static str,
    chamfer: &'static str,
}

const RIM_FIELDS: [RimField; 2] = [
    RimField { key: "rim", round: "f-redrawn-rim-round", chamfer: "f-redrawn-rim-chamfer" },
    RimField { key: "base_rim", round: "f-redrawn-base-round", chamfer: "f-redrawn-base-chamfer" },
];

/// A rim a boss or a pocket has, and its field.
struct RimHeld {
    rim: RimBlend,
    field: &'static RimField,
}

impl RimHeld {
    /// What the field is called, by how the rim is made.
    fn label(&self) -> &'static str {
        if self.rim.cut == RimCut::Round {
            self.field.round
        } else {
            self.field.chamfer
        }
    }

    /// The field at `val`.
    fn param(&self, val: f64) -> CmdParam {
        CmdParam::new(self.label(), self.field.key, val, 0.01, 100000.0)
    }
}

/// The rims of `rims` with their fields, at the face then at the base.
fn rims_held(rims: Rims) -> Vec<RimHeld> {
    let at = [rims.at_face, rims.at_base];
    at.into_iter().zip(RIM_FIELDS.iter()).filter_map(|pair| pair.0.map(|rim| RimHeld { rim, field: pair.1 })).collect()
}

/// The sizes of `rims` as the fields of `cmd` hold them; a field the tool does not show leaves its rim as it was.
fn rims_from(cmd: &qymcad_ui_state::FeatCommand, rims: &mut Rims) {
    for (slot, f) in [&mut rims.at_face, &mut rims.at_base].into_iter().zip(RIM_FIELDS.iter()) {
        if let Some(r) = slot.as_mut().filter(|_| cmd.params.iter().any(|p| p.key == f.key)) {
            r.size = qymcad_ui_state::cmd_val(cmd, f.key);
        }
    }
}

/// TAKE THE TOOL: it waits for a click on a flat face, or takes at once the face selected before it.
pub fn start_sketch_from_face_cmd(pc: &mut PartCtx) {
    let before = *pc.sel;
    pc.cmd.open(pc.armed, qymcad_ui_state::PartTool::SketchFromFace, *pc.mode_3d);
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
        pc.gsel.before.spend_object(); // the face selected is the object of the tool, not a click of it
    }
}

/// Whether a face of a body is flat: every triangle of it turned as the face is.
pub(crate) fn flat(pc: &PartCtx, at: qymcad_ui_state::made_by::FaceAt) -> bool {
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
    // A FACE OF A MESH IS DRAWN BY ITS FITTED OUTLINE, as a sketch only: its chords become the lines, arcs and circles
    // they follow (`add_sketch_of_face_loops`), and there is no exact boss to redraw
    if pc.project.came_from_a_mesh(body) {
        pc.recognise.redraw = Some(RedrawInHand { body, face: key, prism: None, told: qymcad_i18n::tr("msg-sketch-from-face-mesh"), tone: qymcad_ui_state::Tone::Info });
        refresh(pc);
        return;
    }
    let found = qymcad_ui_state::with_kernel(&mut pc.rebuild(), |project, k| project.prism_at(body, key.id, k));
    let told = match &found {
        Ok(p) => {
            let what = match p.kind {
                PrismKind::Boss => "msg-sketch-from-face-boss",
                PrismKind::Pocket => "msg-sketch-from-face-pocket",
                PrismKind::Stock => "msg-sketch-from-face-stock",
            };
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
    let tone = if found.is_ok() { qymcad_ui_state::Tone::Info } else { qymcad_ui_state::Tone::Refusal };
    let prism = found.ok();
    pc.recognise.redraw = Some(RedrawInHand { body, face: key, prism, told, tone });
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
    let mut said = format!("{}. {}", held.told, qymcad_i18n::tr(enter));
    // a copy taken where a boss could be redrawn says how to have the sketch drive it: a person changed the sizes of the
    // copy and waited for the block to follow
    if !replace && held.prism.is_some() {
        said = format!("{said}. {}", qymcad_i18n::tr("msg-sketch-from-face-copy-replace"));
    }
    // with Replace, what cannot be redrawn is refused, and the plate of the tool says it; a copy is taken anyway
    let tone = if pc.feat.op == REPLACE { held.tone } else { qymcad_ui_state::Tone::Info };
    *pc.status = qymcad_ui_state::Notice { tone, words: said };
    match held.prism.as_ref().filter(|_| replace) {
        Some(p) => {
            let given = |key: &str, or: f64| pc.cmd.params.iter().find(|f| f.key == key).map(|f| f.val).unwrap_or(or);
            let mut params = vec![height_field(given("height", p.height))];
            params.extend(rims_held(p.rims).iter().map(|r| r.param(given(r.field.key, r.rim.size))));
            pc.cmd.params = params;
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
pub fn apply_sketch_from_face_cmd(pc: &mut PartCtx) -> Result<Id, qymcad_ui_state::NotApplied> {
    let held = pc.recognise.redraw.clone().ok_or_else(|| super::refused("msg-sketch-from-face"))?;
    let replace = pc.feat.op == REPLACE;
    let height = qymcad_ui_state::cmd_val(pc.cmd, "height");
    let outline = held.prism.as_ref().map(|p| p.outline.clone()).unwrap_or_default();
    let profile = held.prism.as_ref().map(|p| p.profile.clone()).unwrap_or_default();
    let name = qymcad_i18n::tr1("g-sketch-n", "n", &(pc.project.sketches.len() + 1).to_string());
    let mut prism = held.prism.clone().filter(|_| replace);
    if let Some(p) = prism.as_mut() {
        rims_from(pc.cmd, &mut p.rims);
    }
    if pc.project.came_from_a_mesh(held.body) {
        let fit = |pts: &[qymcad_core::geom::Point2]| qymcad_meshfit::section::fit_loop(pts, &qymcad_meshfit::section::Fit::for_loop(pts));
        let Some(sketch) = pc.project.add_sketch_of_face_loops(qymcad_core::model::OfFace { body: held.body, face: held.face, fit: &fit, name }) else {
            return Err(super::refused("msg-sketch-from-face-no-outline"));
        };
        pc.recognise.laid = told_laid(&sketch);
        return Ok(sketch.sketch);
    }
    let laid = qymcad_ui_state::with_kernel(&mut pc.rebuild(), |project, k| {
        let sketch = project.add_sketch_from_face(FromFace { body: held.body, face: held.face, profile: &profile, outline: &outline, name }, k)?;
        let node = prism.as_ref().map(|p| project.add_redrawn(held.body, p, sketch.sketch));
        Some(Laid { sketch, node })
    });
    let Some(Laid { sketch, node }) = laid else {
        return Err(super::refused("msg-sketch-from-face-no-outline"));
    };
    if let Some(node) = node {
        if let Some(FeatureKind::Redrawn { height: h, .. }) = pc.project.timeline.iter_mut().find(|n| n.id == node).map(|n| &mut n.kind) {
            *h = height;
        }
        if let Err(e) = super::store_cmd_exprs(pc.cmd, pc.project, node) {
            return Err(super::formula_refused(&e));
        }
    }
    pc.recognise.laid = told_laid(&sketch);
    Ok(node.unwrap_or(sketch.sketch))
}

/// What a sketch taken off a face laid, in words (`qymcad_ui_state::told_defined`).
fn told_laid(s: &FaceSketch) -> String {
    qymcad_ui_state::told_defined(&s.defined, "msg-sketch-from-face-laid")
}

/// A DOUBLE CLICK ON A BOSS OR POCKET REDRAWN reopens the tool on its height: its faces lit on the body it was taken off,
/// the height in the field - an expression as it was typed - and Enter changes the node rather than laying another. Its
/// outline is the sketch's: a double click on the sketch edits it.
pub fn open_redrawn(pc: &mut PartCtx, fid: Id, kind: &FeatureKind, was_3d: bool) {
    let FeatureKind::Redrawn { src, ref faces, height, rims, sketch, .. } = *kind else { return };
    pc.cmd.open(pc.armed, qymcad_ui_state::PartTool::SketchFromFace, was_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, &mut *pc.view_restore);
    *pc.mode_3d = true;
    pc.recognise.redraw = None;
    pc.feat.op = REPLACE;
    pc.gsel.faces = super::live_picks(&*pc.project, src, faces, true).into_iter().collect();
    pc.gsel.faces_body = Some(src);
    let mut params = vec![super::cmd_param_from(&*pc.project, fid, "f-redrawn-height", "height", height, 0.01, 100000.0)];
    for r in rims_held(rims) {
        params.push(super::cmd_param_from(&*pc.project, fid, r.label(), r.field.key, r.rim.size, 0.01, 100000.0));
    }
    // the main sizes of the outline beside the height: a boss 12 across is made 16 where its height is typed
    let sizes = pc.project.outline_sizes(sketch);
    for (i, s) in sizes.iter().enumerate() {
        let mut f = CmdParam::new(outline_label(s.kind), &outline_key(i), s.value, 0.01, 100000.0);
        if !s.expr.is_empty() {
            f.txt = s.expr.clone();
        }
        params.push(f);
    }
    pc.cmd.params = params;
    pc.cmd.focus = true;
    let told = if sizes.is_empty() { "msg-edit-redrawn" } else { "msg-edit-redrawn-sizes" };
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr(told));
}

/// The height and the rims written into a boss or pocket redrawn being edited; a field the tool does not show leaves its
/// size as it was.
pub fn edit_redrawn(cmd: &qymcad_ui_state::FeatCommand, height: &mut f64, rims: &mut Rims) {
    if cmd.params.iter().any(|p| p.key == "height") {
        *height = qymcad_ui_state::cmd_val(cmd, "height");
    }
    rims_from(cmd, rims);
}

/// The key of the field of the `i`-th main size of an outline.
fn outline_key(i: usize) -> String {
    format!("outline{i}")
}

/// What the field of a main size of an outline is called.
fn outline_label(kind: OutlineSizeKind) -> &'static str {
    match kind {
        OutlineSizeKind::Diameter => "f-redrawn-diameter",
        OutlineSizeKind::Width => "f-redrawn-width",
        OutlineSizeKind::Length => "f-redrawn-length",
    }
}

/// THE MAIN SIZES OF THE OUTLINE OF BOSS OR POCKET `fid` REDRAWN, as their fields hold them, written into the sketch it
/// is drawn by - the same dimension the sketch shows, a formula kept as typed - and the sketch solved again. The fields
/// are not numbers of the node: what `store_cmd_exprs` wrote under their keys is taken back off it.
pub fn edit_outline(pc: &mut PartCtx, fid: Id) -> Result<(), qymcad_ui_state::NotApplied> {
    let Some(FeatureKind::Redrawn { sketch, .. }) = pc.project.timeline.iter().find(|n| n.id == fid).map(|n| n.kind.clone()) else { return Ok(()) };
    let sizes = pc.project.outline_sizes(sketch);
    let mut changed = false;
    for (i, s) in sizes.iter().enumerate() {
        let key = outline_key(i);
        pc.project.clear_feat_formula(&qymcad_core::model::FeatNumber { node: fid, key: &key });
        let Some(typed) = pc.cmd.params.iter().find(|p| p.key == key).map(|p| p.txt.clone()) else { continue };
        if typed.trim() == s.expr.trim() || (s.expr.is_empty() && typed.trim().parse::<f64>().is_ok_and(|v| (v - s.value).abs() < 1e-9)) {
            continue;
        }
        pc.project.set_dim_target_text(&s.target, &typed).map_err(|e| super::formula_refused(&e))?;
        changed = true;
    }
    if changed {
        if let Some(si) = pc.project.sketch_index(sketch) {
            pc.project.solve_sketch(si);
        }
        pc.project.mark_sketch_dirty(sketch);
    }
    Ok(())
}
