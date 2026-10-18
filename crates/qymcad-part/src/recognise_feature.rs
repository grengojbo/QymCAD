//! RECOGNISE FEATURE: a click on a hole, a rounding or a chamfer of a body takes it - the faces it is made of lit, its
//! size in a field at the geometry, brought to what was drawn - and Enter makes it again exact: its faces removed and
//! the feature laid down by its own tool (`Project::add_remade_feature`). Switched in the bar to "Take off", Enter only
//! removes its faces (`Project::add_remove_face`): the corner sharp again, the wall whole. What it is is told by
//! `qymcad_meshfit::feature` from the body's own faces, so a polyhedron made of a mesh, a recognised body and a STEP are
//! clicked alike. The tool stays in hand for the next feature.
use qymcad_core::model::{Exact, Id, Remake};
use qymcad_meshfit::feature::{feature_at, Found, Pick, Reach, Unfound};
use qymcad_ui_state::{CmdParam, FeatureInHand, PartCtx};

/// The code of the command.
pub const RECOGNISE_FEATURE: u8 = 37;

/// What Enter does with the feature taken (the operation of the bar, `FeatParams::op`): makes it again exact, or takes it
/// off the body.
pub const MAKE_EXACT: u8 = 0;
pub const TAKE_OFF: u8 = 1;

/// The tolerance field: how far the body's corners may lie off the surface, as a multiple of the body's own.
fn tolerance_field() -> CmdParam {
    CmdParam::new("f-recognise-tol", "tol", 1.0, 0.1, 1000.0)
}

pub fn start_recognise_feature_cmd(pc: &mut PartCtx) {
    pc.cmd.open(pc.armed, RECOGNISE_FEATURE, *pc.mode_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, pc.view_restore);
    *pc.mode_3d = true;
    pc.gsel.edges.clear();
    pc.gsel.faces.clear();
    pc.gsel.faces_body = None;
    pc.recognise.feature = None;
    pc.feat.op = MAKE_EXACT;
    pc.cmd.params = vec![tolerance_field()];
    // NO FIELD TAKES THE KEYS until a feature is found: the tool waits for a click, and a field in focus kept Ctrl+Z for
    // its own text - the feature just made exact, the tool taken again, could not be undone
    pc.cmd.focus = false;
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-recognise-feature"));
}

/// A CLICK ON FACE `face` OF BODY `mesh` with the tool in hand: the feature there is found and taken, or the status line
/// says why none was.
pub fn recognise_feature_at(pc: &mut PartCtx, mesh: usize, face: Option<usize>, at: Option<[f64; 3]>) {
    let (Some(body), Some(fi)) = (pc.project.mesh_id(mesh), face) else { return };
    let Some(b) = pc.project.bodies.get(mesh) else { return };
    let Some(f) = b.faces.get(fi) else { return };
    // THE FEATURE TAKEN, CLICKED AGAIN, IS LET GO, as any pick is
    if pc.recognise.feature.as_ref().is_some_and(|held| held.body == body) && pc.gsel.faces.contains(&f.id) {
        pc.recognise.feature = None;
        pc.gsel.faces.clear();
        pc.gsel.faces_body = None;
        *pc.sel = qymcad_ui_state::Sel::None;
        pc.cmd.params.retain(|p| p.key == "tol");
        *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-recognise-feature"));
        return;
    }
    let pick = Pick { face: f.id, at: at.unwrap_or([f.centroid.x, f.centroid.y, f.centroid.z]) };
    let tol = qymcad_ui_state::cmd_val(pc.cmd, "tol");
    let found = feature_at(&b.mesh, &b.faces, pick, tol).and_then(|found| found.remake(&b.faces).map(|remake| Taken { found, remake }).ok_or(Unfound::Other));
    pc.gsel.faces.clear();
    let tol_field = pc.cmd.params.iter().find(|p| p.key == "tol").cloned().unwrap_or_else(tolerance_field);
    match found {
        Ok(Taken { found, remake }) => {
            pc.gsel.faces.extend(remake.faces.iter().map(|k| k.id));
            pc.gsel.faces_body = Some(body);
            let told = said(&found, &remake);
            pc.recognise.feature = Some(FeatureInHand { body, remake, told });
            fields_for_mode(pc, tol_field);
        }
        Err(why) => {
            pc.cmd.params = vec![tol_field];
            pc.recognise.feature = None;
            *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr(unfound_key(why)));
        }
    }
}

/// THE OPERATION OF THE BAR CHANGED: the fields and the status line follow it.
pub fn recognise_mode_changed(pc: &mut PartCtx) {
    let tol_field = pc.cmd.params.iter().find(|p| p.key == "tol").cloned().unwrap_or_else(tolerance_field);
    fields_for_mode(pc, tol_field);
}

/// The fields of the feature held as the operation of the bar wants them - its sizes first when it is made exact, since
/// a size is what a person types next and it takes the keys; none when it is taken off - and the status line saying
/// what Enter does.
fn fields_for_mode(pc: &mut PartCtx, tol_field: CmdParam) {
    let Some(held) = pc.recognise.feature.as_ref() else {
        pc.cmd.params = vec![tol_field];
        return;
    };
    let exact = pc.feat.op != TAKE_OFF;
    let sizes = if exact { size_fields(&held.remake) } else { Vec::new() };
    pc.cmd.focus = !sizes.is_empty();
    pc.cmd.params = sizes.into_iter().chain(std::iter::once(tol_field)).collect();
    let then = if exact { "msg-found-then-exact" } else { "msg-found-then-off" };
    *pc.status = qymcad_ui_state::Notice::info(format!("{}. {}", held.told, qymcad_i18n::tr(then)));
}

/// A feature found and what the core is to lay down for it.
struct Taken {
    found: Found,
    remake: Remake,
}

/// A field of a size: its caption, its key - the node's dimension - its value and its bounds.
struct Size {
    label: &'static str,
    key: &'static str,
    val: f64,
    lo: f64,
    hi: f64,
}

/// The sizes a person may change: a hole's diameter, and its depth unless it goes through; a rounding's radius; a
/// chamfer's leg.
fn sizes(exact: &Exact) -> Vec<Size> {
    match *exact {
        Exact::Hole { diameter, depth, reach, .. } => {
            let mut out = vec![Size { label: "f-diameter", key: "diameter", val: diameter, lo: 0.1, hi: 10000.0 }];
            if reach == Reach::Blind {
                out.push(Size { label: "f-depth", key: "depth", val: depth, lo: 0.1, hi: 10000.0 });
            }
            out
        }
        Exact::Round { radius, .. } => vec![Size { label: "f-radius", key: "radius", val: radius, lo: 0.05, hi: 1000.0 }],
        Exact::Chamfer { leg, .. } => vec![Size { label: "f-leg", key: "dist", val: leg, lo: 0.05, hi: 1000.0 }],
    }
}

/// The fields of the sizes of a feature just found, each starting from the size brought to what was drawn.
fn size_fields(remake: &Remake) -> Vec<CmdParam> {
    sizes(&remake.exact).into_iter().map(|z| CmdParam::new(z.label, z.key, z.val, z.lo, z.hi)).collect()
}

/// A DOUBLE CLICK ON A FEATURE MADE EXACT reopens the tool on it: the faces it took lit on the body they were taken from,
/// its sizes in the fields - an expression as it was typed - and Enter changes the node rather than laying another.
pub fn open_remade(pc: &mut PartCtx, fid: Id, src: Id, faces: &qymcad_core::refs::Ref, exact: &Exact, was_3d: bool) {
    pc.cmd.open(pc.armed, RECOGNISE_FEATURE, was_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, &mut *pc.view_restore);
    *pc.mode_3d = true;
    pc.recognise.feature = None;
    pc.gsel.faces = super::live_picks(&*pc.project, src, faces, true);
    pc.gsel.faces_body = Some(src);
    let fields: Vec<CmdParam> = sizes(exact).into_iter().map(|z| super::cmd_param_from(&*pc.project, fid, z.label, z.key, z.val, z.lo, z.hi)).collect();
    pc.cmd.params = fields;
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-edit-remade"));
}

/// The sizes of the fields written into a feature made exact being edited; a field the tool does not show leaves its
/// size as it was.
pub fn edit_remade(cmd: &qymcad_ui_state::FeatCommand, exact: &mut Exact) {
    let field = |key: &str| cmd.params.iter().any(|p| p.key == key).then(|| qymcad_ui_state::cmd_val(cmd, key));
    match exact {
        Exact::Hole { diameter, depth, .. } => {
            *diameter = field("diameter").unwrap_or(*diameter);
            *depth = field("depth").unwrap_or(*depth);
        }
        Exact::Round { radius, .. } => *radius = field("radius").unwrap_or(*radius),
        Exact::Chamfer { leg, .. } => *leg = field("dist").unwrap_or(*leg),
    }
}

/// What was found, in words: the size measured and the size it was brought to.
fn said(found: &Found, remake: &Remake) -> String {
    // the size measured to a hundredth, the size it is brought to as the tree names it
    let (num, size) = (|v: f64| qymcad_i18n::num(v, 2), |v: f64| qymcad_i18n::num(v, 1));
    match (found, &remake.exact) {
        (Found::Hole(h), Exact::Hole { diameter, .. }) => {
            let key = if h.reach == Reach::Through { "msg-found-hole-through" } else { "msg-found-hole-blind" };
            qymcad_i18n::trn(key, &[("measured", &num(h.diameter)), ("size", &size(*diameter)), ("depth", &size(h.depth))])
        }
        (Found::Round(r), Exact::Round { radius, .. }) => qymcad_i18n::tr2("msg-found-round", "measured", &num(r.radius), "size", &size(*radius)),
        (Found::Chamfer(c), Exact::Chamfer { leg, .. }) => qymcad_i18n::tr2("msg-found-chamfer", "measured", &num(c.leg), "size", &size(*leg)),
        _ => String::new(),
    }
}

/// The words for why nothing was found.
fn unfound_key(why: Unfound) -> &'static str {
    match why {
        Unfound::NoSurface => "msg-unfound-no-surface",
        Unfound::NoEntry => "msg-unfound-no-entry",
        Unfound::NotTwoSides => "msg-unfound-not-two-sides",
        Unfound::Uneven => "msg-unfound-uneven",
        Unfound::Other => "msg-unfound-other",
    }
}

/// APPLY: the feature held laid down exact, with the sizes of the fields - a number, or an expression kept on the node;
/// or, taken off, its faces removed.
pub fn apply_recognise_feature_cmd(pc: &mut PartCtx) -> Option<Id> {
    let Some(FeatureInHand { body, mut remake, .. }) = pc.recognise.feature.clone() else {
        *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-recognise-feature"));
        return None;
    };
    if pc.feat.op == TAKE_OFF {
        return Some(pc.project.add_remove_face(body, remake.faces));
    }
    edit_remade(pc.cmd, &mut remake.exact);
    let node = pc.project.add_remade_feature(body, remake);
    if let Err(e) = super::store_cmd_exprs(pc.cmd, pc.project, node) {
        return super::formula_refused(pc, &e);
    }
    Some(node)
}

/// Where the ray through `screen` meets face `face` of body `mesh`, in the body's own frame; `None` where it misses.
pub fn hit_on_face(pc: &PartCtx, mesh: usize, face: Option<usize>, rect: egui::Rect, screen: egui::Pos2) -> Option<[f64; 3]> {
    let body = pc.project.mesh_id(mesh)?;
    let b = pc.project.bodies.get(mesh)?;
    let f = b.faces.get(face?)?;
    let ctx = qymcad_ui_state::current_ctx_id(pc.active_path, pc.project);
    let inv = qymcad_core::feature::mat_inv12(&pc.project.body_display_transform(body, ctx));
    let (o, d) = qymcad_ui_state::screen_ray(pc.cam, rect, screen);
    let ray = Ray { from: qymcad_core::feature::apply12(&inv, o), dir: qymcad_core::feature::apply12_dir(&inv, d) };
    f.triangles.iter().filter_map(|&t| ray.meets(b.mesh.triangle(t as usize).map(|p| [p.x, p.y, p.z]))).min_by(|a, b| a.along.total_cmp(&b.along)).map(|h| h.at)
}

/// A ray: where it starts and which way it goes.
struct Ray {
    from: [f64; 3],
    dir: [f64; 3],
}

/// Where a ray meets a triangle, and how far along it.
struct Hit {
    along: f64,
    at: [f64; 3],
}

impl Ray {
    /// Where the ray meets triangle `t`, either side of it facing.
    fn meets(&self, t: [[f64; 3]; 3]) -> Option<Hit> {
        let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let cross = |a: [f64; 3], b: [f64; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
        let p = cross(self.dir, e2);
        let det = dot(e1, p);
        if det.abs() < 1e-12 {
            return None;
        }
        let s = sub(self.from, t[0]);
        let u = dot(s, p) / det;
        let q = cross(s, e1);
        let v = dot(self.dir, q) / det;
        let along = dot(e2, q) / det;
        ((0.0..=1.0).contains(&u) && v >= 0.0 && u + v <= 1.0 && along >= 0.0)
            .then(|| Hit { along, at: [self.from[0] + self.dir[0] * along, self.from[1] + self.dir[1] * along, self.from[2] + self.dir[2] * along] })
    }
}
