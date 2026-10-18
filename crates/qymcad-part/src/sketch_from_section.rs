//! SKETCH FROM SECTION: a body of a mesh is clicked, the plane is a base plane chosen in the bar or a datum plane or a
//! flat face clicked, moved by an offset, and the section the plane cuts it in is shown as the lines, arcs, circles and
//! splines it is fitted with (`qymcad_meshfit::section`). Enter lays a sketch of them on that plane - a datum plane
//! laid off it where the offset is not none - defined to the end (`Project::add_sketch_of_section`). A mesh has no
//! sketch behind it and no exact edge to project: its section is the drawing a person would trace.
use qymcad_core::feature::{BasePlane, SketchPlane};
use qymcad_core::geom::{Fitted, Piece};
use qymcad_core::model::{Id, OfSection, Project, SectionAt};
use qymcad_meshfit::section::{fit_section, Within};
use qymcad_ui_state::{CmdParam, PartCtx, SectionDrawn, SectionInHand};
use std::cell::RefCell;
use std::rc::Rc;

/// The code of the command.
pub const SKETCH_FROM_SECTION: u8 = 40;

/// The base plane the section is taken off, as the operation of the bar (`FeatTarget::op`) holds it.
pub const ON_XY: u8 = 0;
pub const ON_XZ: u8 = 1;
pub const ON_YZ: u8 = 2;

/// The tolerance a body gives by itself, as a share of its size: 2e-4, never under 0.01 mm (`section::Fit::for_loop`).
const TOL_SHARE: f64 = 2e-4;

/// The base plane the operation of the bar names.
pub fn section_base(op: u8) -> BasePlane {
    match op {
        ON_XZ => BasePlane::XZ,
        ON_YZ => BasePlane::YZ,
        _ => BasePlane::XY,
    }
}

/// The field of the offset, bounded by the body: a plane at or past its ends cuts nothing - or a face lying in it.
fn offset_field(val: f64, m: &Measured) -> CmdParam {
    CmdParam::new("f-section-offset", "offset", val, m.from + INSIDE, m.to - INSIDE)
}

/// How far inside its ends a body is cut at the least: 0.01 mm.
const INSIDE: f64 = 0.01;

fn tol_field(val: f64) -> CmdParam {
    CmdParam::new("f-section-tol", "tol", val, 0.001, 10.0)
}

/// TAKE THE TOOL: it waits for a click on a body of a mesh, or takes at once the body of the face selected before it.
pub fn start_sketch_from_section_cmd(pc: &mut PartCtx) {
    let before = *pc.sel;
    pc.cmd.open(pc.armed, SKETCH_FROM_SECTION, *pc.mode_3d);
    qymcad_ui_state::borrow_view(*pc.cam, *pc.mode_3d, *pc.view, pc.view_restore);
    *pc.mode_3d = true;
    pc.gsel.edges.clear();
    pc.gsel.faces.clear();
    pc.gsel.faces_body = None;
    pc.recognise.section = None;
    pc.feat.op = ON_XY;
    pc.cmd.params = Vec::new();
    pc.cmd.focus = false;
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-sketch-from-section"));
    if let qymcad_ui_state::Sel::Face(mesh, _) = before {
        sketch_from_section_at(pc, mesh, None);
    }
}

/// A number to a tenth: the offset a field opens with reads 5.0, not 4.99999.
fn tenth(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Where a body begins and ends along the way an offset off a base plane moves, its middle, and the tolerance its size
/// gives.
struct Measured {
    from: f64,
    to: f64,
    middle: f64,
    tol: f64,
}

fn measured(project: &Project, body: Id, on: &SketchPlane) -> Option<Measured> {
    let e = project.extent_along(body, on)?;
    let b = project.bodies.get(project.mesh_index(body)?)?.mesh.bounds()?;
    let size = ((b.max.x - b.min.x).powi(2) + (b.max.y - b.min.y).powi(2) + (b.max.z - b.min.z).powi(2)).sqrt();
    Some(Measured { from: e.from, to: e.to, middle: (e.from + e.to) / 2.0, tol: (size * TOL_SHARE).max(qymcad_meshfit::section::LEAST_TOL) })
}

/// The plane the held section is moved off: the one clicked, or the base plane of the bar.
fn section_on(held: SectionInHand, op: u8) -> SketchPlane {
    held.picked.unwrap_or(SketchPlane::World(section_base(op)))
}

/// A CLICK ON A BODY with the tool in hand: a body of a mesh is taken, cut through its middle, the fields of the offset
/// and the tolerance opened; a body of exact surfaces is refused in words that name the tools for it. With a body held,
/// another flat face clicked - of it or of another body - is the plane the section is moved off (`section_plane_taken`);
/// the face the body was taken by, or the body elsewhere, clicked again lets it go.
pub fn sketch_from_section_at(pc: &mut PartCtx, mesh: usize, face: Option<usize>) {
    let Some(body) = pc.project.mesh_id(mesh) else { return };
    if let Some(held) = pc.recognise.section {
        let taken_by = held.body == body && held.by.is_some() && held.by == face;
        let flat_face = face.filter(|&fi| !taken_by && crate::sketch_from_face::flat(pc, qymcad_ui_state::made_by::FaceAt { mesh, face: fi }));
        if let Some(f) = flat_face.and_then(|fi| pc.project.bodies.get(mesh).and_then(|b| b.faces.get(fi))) {
            let key = qymcad_core::feature::FaceKey { index: 0, centroid: [f.centroid.x, f.centroid.y, f.centroid.z], normal: f.normal, id: f.id };
            section_plane_taken(pc, SketchPlane::Face(body, key));
            return;
        }
    }
    if pc.recognise.section.is_some_and(|h| h.body == body) {
        pc.recognise.section = None;
        *pc.sel = qymcad_ui_state::Sel::None;
        refresh(pc);
        return;
    }
    if !pc.project.came_from_a_mesh(body) {
        *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-sketch-from-section-exact"));
        return;
    }
    let Some(m) = measured(pc.project, body, &SketchPlane::World(section_base(pc.feat.op))) else { return };
    pc.recognise.section = Some(SectionInHand { body, by: face, picked: None, drawn: None });
    pc.cmd.params = vec![offset_field(tenth(m.middle), &m), tol_field(m.tol)];
    pc.cmd.focus = true;
    refresh(pc);
}

/// THE PLANE OF THE BAR CHANGED: a plane clicked is let go, and the section moves to the middle of the body along the
/// new plane's way.
pub fn sketch_from_section_plane_changed(pc: &mut PartCtx) {
    let Some(held) = pc.recognise.section else { return };
    pc.recognise.section = Some(SectionInHand { picked: None, ..held });
    to_the_middle(pc);
    refresh(pc);
}

/// A DATUM PLANE OR A FLAT FACE CLICKED with a body held: the section is moved off it, from the middle of the body
/// along its normal. The same plane clicked again is let go, and the bar's base plane is the plane again. With no body
/// held the click is refused: the body comes first.
pub fn section_plane_taken(pc: &mut PartCtx, on: SketchPlane) {
    let Some(held) = pc.recognise.section else {
        *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-sketch-from-section-body-first"));
        return;
    };
    let picked = (held.picked != Some(on)).then_some(on);
    if picked.is_some_and(|p| measured(pc.project, held.body, &p).is_none()) {
        return;
    }
    pc.recognise.section = Some(SectionInHand { picked, ..held });
    to_the_middle(pc);
    refresh(pc);
}

/// The field of the offset opened at the middle of the held body along the way the plane it is moved off moves.
fn to_the_middle(pc: &mut PartCtx) {
    let Some(held) = pc.recognise.section else { return };
    if let Some(m) = measured(pc.project, held.body, &section_on(held, pc.feat.op)) {
        if let Some(f) = pc.cmd.params.iter_mut().find(|f| f.key == "offset") {
            *f = offset_field(tenth(m.middle), &m);
        }
    }
}

/// The body lit and the status line as the body held stands.
fn refresh(pc: &mut PartCtx) {
    let Some(held) = pc.recognise.section else {
        pc.gsel.faces.clear();
        pc.gsel.faces_body = None;
        pc.cmd.params = Vec::new();
        pc.cmd.focus = false;
        *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr("msg-sketch-from-section"));
        return;
    };
    pc.gsel.faces_body = Some(held.body);
    let key = if held.picked.is_some() { "msg-sketch-from-section-plane" } else { "msg-sketch-from-section-enter" };
    *pc.status = qymcad_ui_state::Notice::info(qymcad_i18n::tr(key));
}

/// Whether Enter has what it needs: a body held, and the fields of where and how finely it is cut.
pub fn sketch_from_section_ready(pc: &PartCtx) -> bool {
    pc.recognise.section.is_some() && ["offset", "tol"].iter().all(|k| pc.cmd.params.iter().any(|f| f.key == *k))
}

/// WHERE AND HOW FINELY THE SECTION IS TAKEN: the body, the plane and offset, the tolerance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionAsked {
    pub body: Id,
    pub at: SectionAt,
    pub tol: f64,
}

/// What the tool in hand asks for, by its bar and fields.
pub fn section_asked(held: SectionInHand, op: u8, cmd: &qymcad_ui_state::FeatCommand) -> SectionAsked {
    SectionAsked { body: held.body, at: SectionAt { on: section_on(held, op), offset: qymcad_ui_state::cmd_val(cmd, "offset") }, tol: qymcad_ui_state::cmd_val(cmd, "tol") }
}

/// A preview and what it was worked out for.
struct Remembered {
    asked: SectionAsked,
    verts: usize,
    loops: Rc<Vec<Fitted>>,
}

thread_local! {
    // THE SECTION IS REMEMBERED BETWEEN FRAMES: cut and fitted again only when the body, the plane, the offset or the
    // tolerance changes - a mesh of 200 000 triangles is cut in tens of milliseconds, too much for every frame.
    static PREVIEW: RefCell<Option<Remembered>> = const { RefCell::new(None) };
}

/// THE SECTION AS ENTER WOULD DRAW IT, in the frame of its plane (`SectionAt::frame`).
pub fn section_preview(project: &Project, asked: SectionAsked) -> Rc<Vec<Fitted>> {
    let Some(mesh) = project.mesh_index(asked.body).and_then(|i| project.bodies.get(i)).map(|b| &b.mesh) else { return Rc::default() };
    let verts = mesh.verts.len();
    if let Some(found) = PREVIEW.with(|p| p.borrow().as_ref().filter(|r| r.asked == asked && r.verts == verts).map(|r| r.loops.clone())) {
        return found;
    }
    let Some(frame) = project.section_frame(&asked.at) else { return Rc::default() };
    let loops = Rc::new(fit_section(mesh, &frame, Within::Typed(asked.tol)));
    PREVIEW.with(|p| *p.borrow_mut() = Some(Remembered { asked, verts, loops: loops.clone() }));
    loops
}

/// HOW MANY OF EACH KIND a section holds, in words: "Lines 4, arcs 4, circles 1, splines 0".
pub fn told_pieces(loops: &[Fitted]) -> String {
    let count = |want: fn(&Piece) -> bool| loops.iter().flat_map(|l| &l.pieces).filter(|p| want(p)).count().to_string();
    let lines = qymcad_i18n::tr1("msg-section-lines", "n", &count(|p| matches!(p, Piece::Line { .. })));
    let arcs = qymcad_i18n::tr1("msg-section-arcs", "n", &count(|p| matches!(p, Piece::Arc { .. })));
    let circles = qymcad_i18n::tr1("msg-section-circles", "n", &count(|p| matches!(p, Piece::Circle { .. })));
    let splines = qymcad_i18n::tr1("msg-section-splines", "n", &count(|p| matches!(p, Piece::Spline(_))));
    [lines, arcs, circles, splines].join(", ")
}

/// APPLY: the sketch of the section, on the base plane or a datum plane off it. Answers the sketch; what it holds is
/// told in words for the status line after Enter (`RecogniseTool::laid`). A plane missing the body lays nothing.
pub fn apply_sketch_from_section_cmd(pc: &mut PartCtx) -> Option<Id> {
    let held = pc.recognise.section?;
    let asked = section_asked(held, pc.feat.op, pc.cmd);
    // THE SAME SECTION IS NOT DRAWN TWICE: Enter pressed again, or twice by a heavy hand, laid a second sketch on the first
    let now = SectionDrawn { plane: asked.at.on, offset: asked.at.offset, tol: asked.tol };
    if held.drawn == Some(now) {
        *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-sketch-from-section-drawn"));
        return None;
    }
    let pieces = told_pieces(&section_preview(pc.project, asked));
    let name = qymcad_i18n::tr1("g-sketch-n", "n", &(pc.project.sketches.len() + 1).to_string());
    let tol = asked.tol;
    let fit = move |m: &qymcad_core::geom::Mesh, f: &qymcad_core::feature::PlaneFrame| fit_section(m, f, Within::Typed(tol));
    let Some(made) = pc.project.add_sketch_of_section(OfSection { body: held.body, at: asked.at, fit: &fit, name }) else {
        *pc.status = qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("msg-sketch-from-section-misses"));
        return None;
    };
    pc.recognise.laid = format!("{pieces}; {}", qymcad_ui_state::told_defined(&made.defined, "msg-sketch-from-face-laid"));
    pc.recognise.section = Some(SectionInHand { drawn: Some(now), ..held });
    Some(made.sketch)
}
