//! THE TRIAL BUILD: a value the geometry cannot take is refused beside its field, before Enter.
//!
//! A field knows its own limits (empty, zero, past a bound), but not the ones the body sets: a fillet of 96 on a block
//! 10 thick, a shell as thick as the part, a hole wider than the face, a split plane that misses the body. Those only
//! the kernel answers, so the command is applied, as Enter would apply it, to copies - of the document, of the tool's
//! state and of the part's live bodies - and rebuilt at once. What the new node says is the refusal; nothing of it
//! reaches the document. Reported behaviour: such a value was taken, the node laid, and it went red after the
//! background rebuild.
//!
//! The live bodies are copied through their bytes, names of faces and edges with them: a boolean is allowed to nudge
//! its arguments, and a trial must not move the part it is asked about.
//!
//! The trial runs on a worker, the frames keep coming: while it runs the command says it is checking and does not
//! apply, and the answer takes its place beside the field when it lands. Run on the frame thread a trial of a heavy part
//! stood the window still for as long as the kernel took.
use qymcad_ui_state::Notice;
use qymcad_core::model::Id;
use qymcad_ui_state::PartCtx;
use std::sync::{Arc, Mutex};

/// The commands whose values the geometry bounds and a trial build answers: fillet, chamfer, shell, hole, torus, draft,
/// split body, thicken, split faces, offset surface - a curved face moved in past its radius turns inside out - and a
/// feature made again exact, a rounding of it as wide as its faces allow, and a size between faces. A thread
/// is judged by the checks the rebuild makes before the kernel instead: a trial of a good thread sweeps its turns, 3 s
/// and more a frame in a debug build.
const TRIED: [u8; 13] = [4, 5, 6, 7, 14, 23, 27, 28, 29, 36, 37, 38, super::RECOGNISE_ALL];

pub use qymcad_ui_state::Trial;

/// WHY THE COMMAND AS IT STANDS WOULD NOT BUILD - or that it builds, or that the answer is still coming. A trial runs
/// once per change of a value or a pick, not once a frame, and on a worker.
pub fn trial_refusal(pc: &mut PartCtx, ctx: &egui::Context) -> Trial {
    // the values as typed now: a field takes its text into its value only as it is drawn, after this is asked
    let vars = pc.project.param_map();
    for p in pc.cmd.params.iter_mut() {
        if let Ok(v) = super::field_value(p, &vars) {
            p.val = v;
        }
    }
    let kind = pc.armed.cmd_kind();
    if kind == 24 && !pc.thread.auger && super::cmd_exprs_valid(pc.cmd, pc.project) {
        let t = *pc.thread;
        let length = qymcad_ui_state::cmd_val(pc.cmd, "length");
        let Some(refused) = t.src.and_then(|src| pc.project.thread_refusal(src, (t.axis.0, t.axis.1, t.radius), super::thread_spec(pc.cmd, t), length)) else { return Trial::Clear };
        use qymcad_core::errors::CoreError as E;
        let key = match refused {
            E::ThreadLongerThanFace { .. } | E::ThreadLengthUnset => "length",
            E::ThreadNotItsSize { .. } => "nominal",
            _ => "pitch", // the depth, the fineness and the turns all follow the pitch
        };
        return Trial::Refused(key.to_string(), qymcad_i18n::error_words::error_text(&refused));
    }
    let id = qymcad_ui_state::trial_slot_id();
    if !TRIED.contains(&kind) || pc.live.shapes.is_empty() || !super::cmd_ready(pc) || !super::cmd_exprs_valid(pc.cmd, pc.project) {
        // the last answer goes with the pick it was about: its faces drawn on after the face was let go of
        ctx.data_mut(|d| d.remove::<qymcad_ui_state::TrialSlot>(id));
        return Trial::Clear;
    }
    let key = state_key(pc);
    if let Some(qymcad_ui_state::TrialSlot { key: k, answer }) = ctx.data(|d| d.get_temp::<qymcad_ui_state::TrialSlot>(id)) {
        if k == key {
            qymcad_ui_state::trial_is_current(ctx);
            let got = answer.lock().ok().and_then(|a| a.clone()).map(|a| a.verdict);
            if got.is_none() {
                ctx.request_repaint_after(std::time::Duration::from_millis(30));
                // a frame when the answer lands
            }
            return got.unwrap_or(Trial::Checking);
        }
    }
    let first = pc.cmd.params.first().map(|p| p.key.to_string()).unwrap_or_default();
    let job = gather(pc);
    let answer: Arc<Mutex<Option<qymcad_ui_state::TrialAnswer>>> = Arc::new(Mutex::new(None));
    let out = answer.clone();
    qymcad_kernel::worker(move || {
        // a trial names no field: the command's first one says it - but "Recognise all" names the group that does not lay
        let tried = if kind == super::RECOGNISE_ALL { run_all_trial(job) } else { run_trial(job).beside(first) };
        let (verdict, faces) = (tried.refused.map_or(Trial::Clear, |r| Trial::Refused(r.key, r.words)), tried.faces);
        if let Ok(mut a) = out.lock() {
            *a = Some(qymcad_ui_state::TrialAnswer { verdict, faces: Arc::new(faces) });
        }
    });
    ctx.data_mut(|d| d.insert_temp::<qymcad_ui_state::TrialSlot>(id, qymcad_ui_state::TrialSlot { key, answer }));
    qymcad_ui_state::trial_is_current(ctx);
    ctx.request_repaint_after(std::time::Duration::from_millis(30));
    Trial::Checking
}

/// What the trial depends on: the command, its values and picks, the settings of its bar, and the document.
fn state_key(pc: &PartCtx) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    pc.armed.cmd_kind().hash(&mut h);
    for p in &pc.cmd.params {
        (&p.key, &p.txt).hash(&mut h);
    }
    let mut picks: Vec<String> = pc.gsel.edges.iter().map(|e| format!("{e:?}")).chain(pc.gsel.faces.iter().map(|f| format!("{f:?}"))).collect();
    picks.sort();
    // what "Recognise all" lays is the groups and features it takes; the faces it lights follow the pointer
    match pc.recognise.all.as_ref() {
        Some(all) if pc.armed.cmd_kind() == super::RECOGNISE_ALL => all.groups.iter().map(|g| format!("{:?}{:?}", g.taking, g.off)).collect::<Vec<_>>().hash(&mut h),
        _ => picks.hash(&mut h),
    }
    pc.recognise.gap.hash(&mut h); // the order the faces of a size were taken in: which is the first
    format!("{:?}", (pc.cmd.edit, pc.split.plane, pc.chamfer.mode, *pc.feat, *pc.draft, *pc.hole, *pc.thread, *pc.prim)).hash(&mut h);
    pc.project.state_key().hash(&mut h);
    h.finish()
}

/// EVERYTHING A TRIAL WORKS ON, owned, to be taken to a worker: the document, the tool's state and the bytes of the part's
/// live bodies.
#[derive(Clone)]
struct TrialJob {
    project: qymcad_core::model::Project,
    bodies: Vec<(Id, Vec<u8>)>,
    armed: qymcad_ui_state::Armed,
    cmd: qymcad_ui_state::FeatCommand,
    gsel: qymcad_ui_state::GeomSelection,
    sel: qymcad_ui_state::Sel,
    split: qymcad_ui_state::SplitParams,
    opts: qymcad_ui_state::FeatOptions,
    feat: qymcad_ui_state::FeatTarget,
    mirror: qymcad_ui_state::MirrorParams,
    loft: qymcad_ui_state::LoftParams,
    chamfer: qymcad_ui_state::ChamferParams,
    arr: qymcad_ui_state::ArrayParams,
    datum: qymcad_ui_state::DatumCommand,
    stitch_parts: Vec<Id>,
    prim: qymcad_ui_state::PrimParams,
    draft: qymcad_ui_state::DraftParams,
    hole: qymcad_ui_state::HoleCommand,
    sweep: qymcad_ui_state::SweepParams,
    repl_surface: Option<Id>,
    thread: qymcad_ui_state::ThreadParams,
    params_seen: std::collections::HashMap<String, f64>,
    boolean: qymcad_ui_state::BoolCommand,
    bar_exprs: std::collections::HashMap<&'static str, String>,
    rev: qymcad_ui_state::RevolveParams,
    carr: qymcad_ui_state::CompArrayCmd,
    view: qymcad_ui_state::View2d,
    view_restore: Option<(bool, qymcad_ui_state::Cam3, qymcad_ui_state::View2d)>,
    cam: qymcad_ui_state::Cam3,
    picking: qymcad_ui_state::Picking,
    set: qymcad_ui_state::Settings,
    active_path: Vec<Id>,
    comp_giz: qymcad_ui_state::CompGizmo,
    mode_3d: bool,
    body_giz: qymcad_ui_state::BodyGizmo,
    joint: qymcad_ui_state::JointCommand,
    workbench: qymcad_ui_state::Workbench,
    sketch_ses: qymcad_ui_state::SketchSession,
    /// The feature a "Recognise feature" holds and the faces a "Size on body" took, what of the recognition tools a
    /// trial applies.
    feature: Option<qymcad_ui_state::FeatureInHand>,
    gap: Vec<u32>,
    /// What "Recognise all" found and takes.
    all: Option<qymcad_ui_state::AllInHand>,
}

/// Copies of what the command works on, gathered on the frame thread.
fn gather(pc: &PartCtx) -> TrialJob {
    // the live bodies of the part being worked, as bytes; the others take no part in a command of this part
    let part = qymcad_ui_state::current_ctx_id(pc.active_path, pc.project);
    let bodies = pc
        .live
        .shapes
        .iter()
        .filter(|(id, _)| pc.project.body_owner(**id) == Some(part))
        .filter_map(|(id, shape)| pc.live.blobs.get(id).cloned().or_else(|| shape.to_brep_bytes()).map(|b| (*id, b)))
        .collect();
    TrialJob {
        project: pc.project.clone_without_source_data(),
        bodies,
        armed: *pc.armed,
        cmd: pc.cmd.clone(),
        gsel: pc.gsel.clone(),
        sel: *pc.sel,
        split: pc.split.clone(),
        opts: *pc.opts,
        feat: *pc.feat,
        mirror: pc.mirror.clone(),
        loft: pc.loft.clone(),
        chamfer: *pc.chamfer,
        arr: *pc.arr,
        datum: pc.datum.clone(),
        stitch_parts: pc.stitch_parts.clone(),
        prim: *pc.prim,
        draft: *pc.draft,
        hole: *pc.hole,
        sweep: *pc.sweep,
        repl_surface: *pc.repl_surface,
        thread: *pc.thread,
        params_seen: pc.params_seen.clone(),
        boolean: *pc.boolean,
        bar_exprs: pc.bar_exprs.clone(),
        rev: *pc.rev,
        carr: pc.carr.clone(),
        view: *pc.view,
        view_restore: *pc.view_restore,
        cam: *pc.cam,
        picking: *pc.picking,
        set: pc.set.clone(),
        active_path: pc.active_path.clone(),
        comp_giz: *pc.comp_giz,
        mode_3d: *pc.mode_3d,
        body_giz: pc.body_giz.clone(),
        joint: pc.joint.clone(),
        workbench: pc.workbench,
        sketch_ses: *pc.sketch_ses,
        feature: pc.recognise.feature.clone(),
        gap: pc.recognise.gap.clone(),
        all: pc.recognise.all.clone(),
    }
}

/// Apply the command to the copies and rebuild them at once, on the worker: the words of what refused, if anything did,
/// and the triangles of the faces the new node adds - those its body has and the body it works on has not - in the frame
/// of the context, for the preview.
fn run_trial(job: TrialJob) -> Tried {
    let TrialJob {
        mut project,
        bodies,
        mut armed,
        mut cmd,
        mut gsel,
        mut sel,
        mut split,
        mut opts,
        mut feat,
        mut mirror,
        mut loft,
        mut chamfer,
        mut arr,
        mut datum,
        mut stitch_parts,
        mut prim,
        mut draft,
        mut hole,
        mut sweep,
        mut repl_surface,
        mut thread,
        mut params_seen,
        mut boolean,
        mut bar_exprs,
        mut rev,
        mut carr,
        mut view,
        mut view_restore,
        mut cam,
        mut picking,
        set,
        active_path,
        comp_giz,
        mut mode_3d,
        mut body_giz,
        joint,
        workbench,
        mut sketch_ses,
        feature,
        gap,
        all,
    } = job;
    let before: std::collections::HashSet<Id> = project.timeline.iter().map(|n| n.id).collect();
    let mut live = qymcad_ui_state::LiveGeom { ready: true, ..Default::default() };
    // the copies of the bodies, read back by the bridge under the lock of the kernel
    let saved = bodies.into_iter().map(|(body, bytes)| qymcad_kernel::imports::SavedBody { body, bytes }).collect();
    live.shapes.extend(qymcad_kernel::imports::restore(saved).into_iter().map(|r| (r.body, r.shape)));
    // a rebuild outside a running window happens at once, here, rather than being handed to a thread of its own
    let (mut regen, mut edits, mut status, mut cmd_failed) = (qymcad_ui_state::Rebuilding::default(), qymcad_ui_state::Edits::default(), Notice::default(), false);
    let (mut trim, mut edges, mut cache, mut win) = (Default::default(), Default::default(), Default::default(), Default::default());
    let mut recognise = qymcad_ui_state::RecogniseTool { feature, gap, all, ..Default::default() };
    let scheme = qymcad_ui_state::SchemeUi::default();
    let edited = cmd.edit;
    let mut trial = PartCtx {
        armed: &mut armed,
        status: &mut status,
        cmd: &mut cmd,
        project: &mut project,
        gsel: &mut gsel,
        sel: &mut sel,
        split: &mut split,
        opts: &mut opts,
        feat: &mut feat,
        mirror: &mut mirror,
        loft: &mut loft,
        chamfer: &mut chamfer,
        arr: &mut arr,
        datum: &mut datum,
        stitch_parts: &mut stitch_parts,
        recognise: &mut recognise,
        trim: &mut trim,
        prim: &mut prim,
        draft: &mut draft,
        hole: &mut hole,
        sweep: &mut sweep,
        repl_surface: &mut repl_surface,
        thread: &mut thread,
        edges: &mut edges,
        edits: &mut edits,
        live: &mut live,
        regen: &mut regen,
        params_seen: &mut params_seen,
        boolean: &mut boolean,
        bar_exprs: &mut bar_exprs,
        rev: &mut rev,
        cmd_failed: &mut cmd_failed,
        carr: &mut carr,
        view: &mut view,
        view_restore: &mut view_restore,
        cam: &mut cam,
        picking: &mut picking,
        set: &set,
        active_path: &active_path,
        scheme: &scheme,
        comp_giz: &comp_giz,
        mode_3d: &mut mode_3d,
        body_giz: &mut body_giz,
        joint: &joint,
        workbench,
        cache: &mut cache,
        sketch_ses: &mut sketch_ses,
        win: &mut win,
    };
    super::apply_feat_cmd(&mut trial);
    if cmd_failed {
        return Tried::refused(status.words);
    }
    let laid: Vec<Id> = project.timeline.iter().map(|n| n.id).filter(|id| !before.contains(id) || Some(*id) == edited).collect();
    // a command that laid nothing refused by itself, and its status says why (a split plane that cuts nothing)
    if laid.is_empty() {
        return Tried::refused(status.words);
    }
    if let Some(e) = laid.iter().find_map(|id| project.regen_errors.get(id).filter(|e| !e.retryable())) {
        return Tried { group: e.group(), ..Tried::refused(qymcad_i18n::error_words::error_text(e)) };
    }
    Tried { refused: None, faces: new_faces(&project, &active_path, &laid), group: None }
}

/// The triangles of the faces the nodes `laid` add: the faces of their bodies that the bodies they work on do not have.
fn new_faces(project: &qymcad_core::model::Project, active_path: &[Id], laid: &[Id]) -> Vec<[[f64; 3]; 3]> {
    let ctx = qymcad_ui_state::current_ctx_id(active_path, project);
    let mut out = Vec::new();
    for n in project.timeline.iter().filter(|n| laid.contains(&n.id)) {
        let had: std::collections::HashSet<u32> = n.kind.consumed_body().and_then(|s| project.mesh_index(s)).map(|mi| project.bodies[mi].faces.iter().map(|f| f.id).collect()).unwrap_or_default();
        for b in n.kind.bodies() {
            let Some(mi) = project.mesh_index(b) else { continue };
            let (body, wt) = (&project.bodies[mi], project.body_display_transform(b, ctx));
            for f in body.faces.iter().filter(|f| !had.contains(&f.id)) {
                for &t in &f.triangles {
                    let tri = body.mesh.triangle(t as usize);
                    out.push(tri.map(|v| qymcad_core::feature::apply12(&wt, [v.x, v.y, v.z])));
                }
            }
        }
    }
    out
}

/// What a trial came to: the field refused and why, if anything was, the faces the new node adds, and the group of a
/// node made exact the refusal stands in.
#[derive(Default)]
struct Tried {
    refused: Option<Refused>,
    faces: Vec<[[f64; 3]; 3]>,
    group: Option<u32>,
}

/// A refusal of a trial: the key of the field it is said beside, and its words.
struct Refused {
    key: String,
    words: String,
}

impl Tried {
    /// Refused in `words`, beside no field yet.
    fn refused(words: String) -> Tried {
        Tried { refused: Some(Refused { key: String::new(), words }), faces: Vec::new(), group: None }
    }

    /// The refusal said beside field `key`.
    fn beside(mut self, key: String) -> Tried {
        if let Some(r) = self.refused.as_mut() {
            r.key = key;
        }
        self
    }
}

/// THE TRIAL OF "RECOGNISE ALL": the groups taken laid together. Refused in a group - the rebuild tells which, its
/// features laid after the faces of all of them came off - that group is refused, beside its first field, with the
/// kernel's words. Refused before any group, as the faces come off, each group is left out in turn, and the first whose
/// leaving out lets the rest lay is the one refused. A chain of roundings is not tried group by group alone: one rounding
/// of a chain taken off alone has another rounding for a neighbour, and is refused for that. Refused with no group to
/// blame, the words stand beside no field.
fn run_all_trial(job: TrialJob) -> Tried {
    let whole = run_trial(job.clone());
    let (Some(refused), Some(all)) = (whole.refused.as_ref(), job.all.as_ref()) else { return whole };
    let taken: Vec<usize> = all.groups.iter().enumerate().filter(|(_, g)| g.taking == qymcad_ui_state::Taking::In && g.off.len() < g.items.len()).map(|(i, _)| i).collect();
    let blamed = |g: Option<usize>| Tried { refused: Some(Refused { key: first_key(all, g), words: refused.words.clone() }), faces: Vec::new(), group: None };
    // the node numbers only the groups that go in: its group k is the k-th of them
    if let Some(g) = whole.group.and_then(|k| taken.get(k as usize).copied()) {
        return blamed(Some(g));
    }
    if taken.len() < 2 {
        return blamed(taken.first().copied());
    }
    for &g in &taken {
        let mut without = job.clone();
        if let Some(gr) = without.all.as_mut().and_then(|a| a.groups.get_mut(g)) {
            gr.taking = qymcad_ui_state::Taking::Out;
        }
        if run_trial(without).refused.is_none() {
            return blamed(Some(g));
        }
    }
    whole
}

/// The key of the first field of group `g`, as the tool keys it.
fn first_key(all: &qymcad_ui_state::AllInHand, g: Option<usize>) -> String {
    let Some((g, exact)) = g.and_then(|g| all.groups.get(g).and_then(|gr| gr.items.first()).map(|r| (g, &r.exact))) else { return String::new() };
    super::recognise_feature::sizes(exact).first().map(|z| qymcad_core::model::group_key(z.key, g as u32).into_owned()).unwrap_or_default()
}
