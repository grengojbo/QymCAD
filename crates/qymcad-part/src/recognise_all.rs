//! RECOGNISE ALL: a body clicked - or the body of the face selected before - and every hole, rounding and chamfer of it
//! is found (`qymcad_meshfit::feature::features_of`), in groups of one size, each group a row beside the body: ticked,
//! its features lit, its sizes in fields. A click on a feature in the view leaves it out or takes it back. Enter lays
//! one node (`Project::add_remade_features`): the faces of all of them taken off at once and each made again by its own
//! tool. A group the kernel cannot lay is found by the trial, left out and named with its reason, and the rest goes in.
use qymcad_core::model::{group_key, Exact, Id, Reach, RemadeItem, Remake, RemakeGroup};
use qymcad_meshfit::feature::{features_of, grouped};
use qymcad_ui_state::{AllInHand, CmdParam, GroupInHand, PartCtx, Taking};

/// The code of the command.
pub const RECOGNISE_ALL: u8 = 41;

/// TAKE THE TOOL: it waits for a click on a body, or takes at once the body of the face selected before it.
pub fn start_recognise_all_cmd(pc: &mut PartCtx) {
    let before = *pc.sel;
    pc.cmd.open(pc.armed, RECOGNISE_ALL, *pc.mode_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, pc.view_restore);
    *pc.mode_3d = true;
    pc.gsel.edges.clear();
    pc.gsel.faces.clear();
    pc.gsel.faces_body = None;
    pc.recognise.all = None;
    pc.cmd.params = Vec::new();
    pc.cmd.focus = false;
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-recognise-all"));
    if let qymcad_ui_state::Sel::Face(mesh, _) | qymcad_ui_state::Sel::Mesh(mesh) = before {
        recognise_all_at(pc, mesh, None);
    }
}

/// A CLICK ON BODY `mesh` with the tool in hand: with nothing held the body is taken and everything on it found; with
/// the body held, the feature whose face was clicked is left out, or taken back. The body is let go, as any pick is, by a
/// click on it off every feature, or by the click that leaves out the last feature taken.
pub fn recognise_all_at(pc: &mut PartCtx, mesh: usize, face: Option<usize>) {
    let Some(body) = pc.project.mesh_id(mesh) else { return };
    let Some(b) = pc.project.bodies.get(mesh) else { return };
    if pc.cmd.edit.is_some() {
        return; // a node reopened changes its sizes; what it holds was chosen when it was laid
    }
    if pc.recognise.all.as_ref().is_some_and(|all| all.body == body) {
        let id = face.and_then(|fi| b.faces.get(fi)).map(|f| f.id);
        let still = pc.recognise.all.as_mut().is_some_and(|all| id.is_some_and(|id| switch_feature(all, id)) && all.groups.iter().any(|gr| taken(gr).next().is_some()));
        if !still {
            let_go(pc);
            return;
        }
        light(pc);
        say_found(pc);
        return;
    }
    let everything = features_of(&b.mesh, &b.faces, 1.0);
    let groups = grouped(&everything.found, &b.faces);
    if groups.is_empty() {
        pc.recognise.all = None;
        pc.cmd.params = Vec::new();
        pc.gsel.faces.clear();
        *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-recognise-all-none"));
        return;
    }
    let groups = groups.into_iter().map(|g| GroupInHand { items: g.items, off: Vec::new(), taking: Taking::In }).collect();
    pc.recognise.all = Some(AllInHand { body, groups, unfound: everything.unfound.len(), hover: None });
    pc.cmd.params = fields_of(pc.recognise.all.as_ref());
    light(pc);
    say_found(pc);
}

/// The body let go: the tool waits for a click on a body again.
fn let_go(pc: &mut PartCtx) {
    pc.recognise.all = None;
    pc.cmd.params = Vec::new();
    pc.gsel.faces.clear();
    pc.gsel.faces_body = None;
    *pc.sel = qymcad_ui_state::Sel::None;
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-recognise-all"));
}

/// The feature holding face `id` left out of its group, or taken back; a group whose every feature is left out is
/// left out, and one taken back is taken again. Answers whether a feature holds the face.
fn switch_feature(all: &mut AllInHand, id: u32) -> bool {
    let Some((g, i)) = all.groups.iter().enumerate().find_map(|(g, gr)| gr.items.iter().position(|r| r.faces.iter().any(|f| f.id == id)).map(|i| (g, i))) else { return false };
    let gr = &mut all.groups[g];
    match gr.off.iter().position(|&k| k == i) {
        Some(at) => {
            gr.off.remove(at);
            gr.taking = Taking::In;
        }
        None if gr.taking != Taking::In => {
            gr.off = (0..gr.items.len()).filter(|&k| k != i).collect();
            gr.taking = Taking::In;
        }
        None => {
            gr.off.push(i);
            if gr.off.len() == gr.items.len() {
                gr.taking = Taking::Out;
            }
        }
    }
    true
}

/// The features of a group that go in: those of a group taken, less those a click left out.
fn taken(gr: &GroupInHand) -> impl Iterator<Item = &Remake> {
    gr.items.iter().enumerate().filter(move |(i, _)| gr.taking == Taking::In && !gr.off.contains(i)).map(|(_, r)| r)
}

/// THE FACES LIT: those of every feature that goes in, or of the group the pointer stands on alone.
pub fn light(pc: &mut PartCtx) {
    let Some(all) = pc.recognise.all.as_ref().filter(|_| pc.cmd.edit.is_none()) else { return };
    let lit: Vec<u32> = match all.hover.and_then(|g| all.groups.get(g)) {
        Some(gr) => gr.items.iter().flat_map(|r| r.faces.iter().map(|f| f.id)).collect(),
        None => all.groups.iter().flat_map(taken).flat_map(|r| r.faces.iter().map(|f| f.id)).collect(),
    };
    pc.gsel.faces = lit.into_iter().collect();
    pc.gsel.faces_body = Some(all.body);
}

/// The fields of the sizes of every group, each starting from the size brought to what was drawn, keyed by the group.
fn fields_of(all: Option<&AllInHand>) -> Vec<CmdParam> {
    let Some(all) = all else { return Vec::new() };
    all.groups
        .iter()
        .enumerate()
        .filter_map(|(g, gr)| gr.items.first().map(|r| (g, r)))
        .flat_map(|(g, r)| crate::recognise_feature::sizes(&r.exact).into_iter().map(move |z| CmdParam::new(z.label, &group_key(z.key, g as u32), z.val, z.lo, z.hi)))
        .collect()
}

/// The status line: how many of each were found and are taken, the curved faces left as they are, and what a click and
/// Enter do.
fn say_found(pc: &mut PartCtx) {
    let Some(all) = pc.recognise.all.as_ref() else { return };
    let count = |of: fn(&Exact) -> bool| all.groups.iter().flat_map(taken).filter(|r| of(&r.exact)).count().to_string();
    let (holes, chamfers, rounds) = (count(|e| matches!(e, Exact::Hole { .. })), count(|e| matches!(e, Exact::Chamfer { .. })), count(|e| matches!(e, Exact::Round { .. })));
    let arg = |name, value| qymcad_i18n::Arg { name, value };
    let mut words = qymcad_i18n::tr_with("msg-recognise-all-found", &[arg("holes", &holes), arg("chamfers", &chamfers), arg("rounds", &rounds)]);
    if all.unfound > 0 {
        words = format!("{words}. {}", qymcad_i18n::tr1("msg-recognise-all-unfound", "n", &all.unfound.to_string()));
    }
    *pc.status = qymcad_ui_state::Notice::info(words);
}

/// What a group is, in words: its kind, its size and how many of it go in.
pub fn group_told(gr: &GroupInHand) -> String {
    // a group left out still says what it holds
    let n = if gr.taking == Taking::In { taken(gr).count() } else { gr.items.len() }.to_string();
    let size = |v: f64| qymcad_i18n::num(v, 1);
    match gr.items.first().map(|r| &r.exact) {
        Some(Exact::Hole { diameter, reach: Reach::Through, .. }) => qymcad_i18n::tr2("msg-all-hole-through", "d", &size(*diameter), "n", &n),
        Some(Exact::Hole { diameter, depth, .. }) => {
            let (d, depth) = (size(*diameter), size(*depth));
            qymcad_i18n::tr_with("msg-all-hole-blind", &[qymcad_i18n::Arg { name: "d", value: &d }, qymcad_i18n::Arg { name: "depth", value: &depth }, qymcad_i18n::Arg { name: "n", value: &n }])
        }
        Some(Exact::Round { radius, .. }) => qymcad_i18n::tr2("msg-all-round", "r", &size(*radius), "n", &n),
        Some(Exact::Chamfer { leg, .. }) => qymcad_i18n::tr2("msg-all-chamfer", "d", &size(*leg), "n", &n),
        None => String::new(),
    }
}

/// THE GROUP WHOSE FIELDS BEGIN AT FIELD `i` of the command, if one does: the row of the list drawn above them.
pub fn group_at_field(params: &[CmdParam], i: usize) -> Option<usize> {
    let g = group_of_key(&params.get(i)?.key)?;
    let before = i.checked_sub(1).and_then(|j| params.get(j)).and_then(|p| group_of_key(&p.key));
    (before != Some(g)).then_some(g)
}

/// The group a field's key names: none for the first group's bare name, the number after the dot less one for the next.
fn group_of_key(key: &str) -> Option<usize> {
    match key.split_once('.') {
        Some((_, n)) => n.parse::<usize>().ok().and_then(|n| n.checked_sub(1)),
        None => Some(0),
    }
}

/// THE ROW OF GROUP `g` in the list beside the body: its tick, what it is, and - left out by the trial - why. The pointer
/// on it lights the group alone. A node reopened shows what it holds, without ticks.
pub fn group_row(pc: &mut PartCtx, ui: &mut egui::Ui, g: usize) {
    let edit = pc.cmd.edit.is_some();
    let pal = pc.scheme.pal.clone();
    let Some(all) = pc.recognise.all.as_mut() else { return };
    let Some(gr) = all.groups.get_mut(g) else { return };
    let told = group_told(gr);
    let row = ui.horizontal(|ui| {
        if edit {
            ui.label(egui::RichText::new(&told).strong());
            return false;
        }
        let mut ticked = gr.taking == Taking::In;
        let changed = ui.checkbox(&mut ticked, "").changed();
        ui.label(egui::RichText::new(&told).strong());
        if changed {
            gr.taking = if ticked { Taking::In } else { Taking::Out };
            gr.off.clear();
        }
        changed
    });
    if let Taking::Refused(why) = &gr.taking {
        qymcad_ui_state::say(ui, &pal, &qymcad_ui_state::Notice::refusal(why.clone()), qymcad_ui_state::NoticeSize::Small);
    }
    if row.response.hovered() || row.response.contains_pointer() {
        all.hover = Some(g);
    }
    if row.inner {
        say_found(pc);
    }
}

/// THE TRIAL'S ANSWER read on the frame: a group it found not laying is left out with its words, and the rest is tried
/// again on the next frame.
pub fn trial_said(pc: &mut PartCtx, trial: &crate::Trial) {
    let crate::Trial::Refused(key, words) = trial else { return };
    let Some(g) = group_of_key(key) else { return };
    let Some(all) = pc.recognise.all.as_mut() else { return };
    let Some(gr) = all.groups.get_mut(g).filter(|gr| gr.taking == Taking::In) else { return };
    gr.taking = Taking::Refused(qymcad_i18n::tr2("msg-recognise-all-left-out", "group", &group_told(gr), "why", words));
    light(pc);
}

/// Whether there is something for Enter to lay: a group taken with a feature in it.
pub fn recognise_all_ready(pc: &PartCtx) -> bool {
    pc.recognise.all.as_ref().is_some_and(|all| all.groups.iter().any(|gr| taken(gr).next().is_some()))
}

/// THE GROUPS THAT GO IN, the sizes of their fields written into their features, and the field of each under the key of
/// its group in the node - a group left out is no group of the node, and the next ones move up.
struct Laid {
    groups: Vec<RemakeGroup>,
    fields: Vec<CmdParam>,
}

fn laid(pc: &PartCtx, all: &AllInHand) -> Laid {
    let mut groups = Vec::new();
    let mut fields = Vec::new();
    for (g, gr) in all.groups.iter().enumerate() {
        let items: Vec<Remake> = taken(gr)
            .map(|r| {
                let mut r = r.clone();
                crate::recognise_feature::edit_exact(pc.cmd, &mut r.exact, g as u32);
                r
            })
            .collect();
        if items.is_empty() {
            continue;
        }
        let k = groups.len() as u32;
        for p in pc.cmd.params.iter().filter(|p| group_of_key(&p.key) == Some(g)) {
            let base = p.key.split_once('.').map_or(p.key.as_str(), |(b, _)| b);
            let Some(base) = ["diameter", "depth", "radius", "dist"].into_iter().find(|b| *b == base) else { continue };
            fields.push(CmdParam { key: group_key(base, k).into_owned(), ..p.clone() });
        }
        groups.push(RemakeGroup { items });
    }
    Laid { groups, fields }
}

/// APPLY: the groups taken laid down exact as one node, with the sizes of the fields - a number, or an expression kept
/// on the node under the key of its group. A node reopened takes its sizes back through its own edit.
pub fn apply_recognise_all_cmd(pc: &mut PartCtx) -> Option<Id> {
    let Some(all) = pc.recognise.all.clone() else {
        *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-recognise-all"));
        return None;
    };
    let Laid { groups, fields } = laid(pc, &all);
    if groups.is_empty() {
        *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-recognise-all-nothing-taken"));
        *pc.cmd_failed = true;
        return None;
    }
    let node = pc.project.add_remade_features(all.body, groups);
    let mut cmd = pc.cmd.clone();
    cmd.params = fields;
    if let Err(e) = super::store_cmd_exprs(&cmd, pc.project, node) {
        return super::formula_refused(pc, &e);
    }
    Some(node)
}

/// A NODE OF FEATURES MADE EXACT TO REOPEN: its id, the body its features were found on, the features, and whether the
/// view was in 3D before.
pub struct RemadeNode<'a> {
    pub fid: Id,
    pub src: Id,
    pub items: &'a [RemadeItem],
    pub was_3d: bool,
}

/// A DOUBLE CLICK ON A NODE OF MANY FEATURES MADE EXACT reopens the list on it: a row a group, its sizes in the fields -
/// an expression as it was typed - and Enter changes the node rather than laying another.
pub fn open_remade_all(pc: &mut PartCtx, node: RemadeNode) {
    let RemadeNode { fid, src, items, was_3d } = node;
    pc.cmd.open(pc.armed, RECOGNISE_ALL, was_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, &mut *pc.view_restore);
    *pc.mode_3d = true;
    let mut groups: Vec<GroupInHand> = Vec::new();
    for it in items {
        let g = it.group as usize;
        while groups.len() <= g {
            groups.push(GroupInHand { items: Vec::new(), off: Vec::new(), taking: Taking::In });
        }
        groups[g].items.push(Remake { faces: Vec::new(), exact: it.exact.clone() });
    }
    pc.recognise.all = Some(AllInHand { body: src, groups, unfound: 0, hover: None });
    pc.gsel.faces = items.iter().flat_map(|it| super::live_picks(&*pc.project, src, &it.faces, true)).collect();
    pc.gsel.faces_body = Some(src);
    let mut seen = std::collections::HashSet::new();
    pc.cmd.params = items
        .iter()
        .filter(|it| seen.insert(it.group))
        .flat_map(|it| {
            crate::recognise_feature::sizes(&it.exact).into_iter().map(|z| super::cmd_param_from(&*pc.project, fid, z.label, &group_key(z.key, it.group), z.val, z.lo, z.hi)).collect::<Vec<_>>()
        })
        .collect();
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-edit-remade"));
}
