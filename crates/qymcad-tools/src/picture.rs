//! THE DOCUMENT AS A PICTURE, WITH NO WINDOW: the scene the window draws, every interface state at rest (nothing in
//! hand, nothing hovered, no tool open), the camera on a standard view and fitted to the model, the bodies drawn by the
//! window's own software rasteriser and written as PNG.
//!
//! The same rasteriser, the same fit and the same angles as the window: a picture here is what a person opening the
//! document sees from that side, less the overlays (sketch lines, gizmos, the grid) - and with the sharp edges of the
//! bodies drawn over them, hidden where a body stands in front. Without them two flat faces facing one way are one
//! colour: from above, the floor of a tray and its rim are a single rectangle.

use std::collections::HashSet;

use egui::{Color32, Rect};
use qymcad_core::model::{Id, Project};
use qymcad_ui_state::{Cam3, Painting, Projection, Sel, Settings};

/// The side the model is looked at from.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    /// From the front, the right and above: the window's opening view.
    #[default]
    Iso,
    Top,
    Bottom,
    /// From -Y, the side the window calls the front.
    Front,
    Back,
    Left,
    Right,
}

impl View {
    /// Where the camera stands, as a direction from the model.
    fn from(self) -> [f64; 3] {
        match self {
            View::Iso => [1.0, -1.0, 1.0],
            View::Top => [0.0, 0.0, 1.0],
            View::Bottom => [0.0, 0.0, -1.0],
            View::Front => [0.0, -1.0, 0.0],
            View::Back => [0.0, 1.0, 0.0],
            View::Left => [-1.0, 0.0, 0.0],
            View::Right => [1.0, 0.0, 0.0],
        }
    }
}

/// The size of a picture in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// What to draw: from which side, how large, which bodies to light as selected, and whether the edges are drawn.
pub struct Look {
    pub view: View,
    pub size: Size,
    pub lit: Lit,
    pub edges: Edges,
}

/// Whether the sharp edges of the bodies are drawn over them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edges {
    Drawn,
    Left,
}

/// An edge in world coordinates, as a polyline.
pub struct Line {
    pub points: Vec<[f64; 3]>,
}

/// THE SHARP EDGES of every body the picture shows, placed in the world. A smooth edge - the seam of a cylinder, the
/// tangent rim of a rounding - is left out: it is no line on the body a person sees.
pub fn lines(doc: &qymcad_doc::DocEngine) -> Vec<Line> {
    let project = doc.project();
    let consumed = project.consumed_bodies();
    let mut out = Vec::new();
    for b in project.bodies.iter().filter(|b| b.visible && !consumed.contains(&b.id)) {
        let Some(shape) = doc.shape(b.id) else { continue };
        let wt = project.body_world_transform(b.id);
        for e in shape.edges_info().into_iter().filter(|e| !e.smooth && e.poly.len() > 1) {
            let points = e.poly.iter().map(|p| qymcad_core::feature::apply12(&wt, [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])])).collect();
            out.push(Line { points });
        }
    }
    out
}

/// The bodies drawn lit, as the window lights a selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lit {
    Nothing,
    Body(Id),
    Part(Id),
}

/// Every interface state the drawing reads, at rest. Held here so that a `Painting` can borrow it.
#[derive(Default)]
struct AtRest {
    armed: qymcad_ui_state::Armed,
    boolean: qymcad_ui_state::BoolCommand,
    active_path: Vec<Id>,
    body_giz: qymcad_ui_state::BodyGizmo,
    cache: qymcad_ui_state::Caches,
    carr: qymcad_ui_state::CompArrayCmd,
    clip: qymcad_ui_state::Clipboard,
    cmd: qymcad_ui_state::FeatCommand,
    datum: qymcad_ui_state::DatumCommand,
    edges: qymcad_ui_state::EdgeCache,
    gsel: qymcad_ui_state::GeomSelection,
    interference: qymcad_ui_state::Interference,
    joint: qymcad_ui_state::JointCommand,
    live: qymcad_ui_state::LiveGeom,
    loft: qymcad_ui_state::LoftParams,
    m3: qymcad_ui_state::Measure3,
    mirror: qymcad_ui_state::MirrorParams,
    pending_import: qymcad_ui_state::PendingImport,
    regen: qymcad_ui_state::Rebuilding,
    rot: qymcad_ui_state::RotInput,
    scheme: qymcad_ui_state::SchemeUi,
    section: qymcad_ui_state::SectionTool,
    sel_sk: qymcad_ui_state::SketchSelection,
    set: Settings,
    sketch_hidden: HashSet<Id>,
    split: qymcad_ui_state::SplitParams,
    stitch_parts: Vec<Id>,
    recognise: qymcad_ui_state::RecogniseTool,
    tool: qymcad_ui_state::SketchTool,
    tool_prefs: qymcad_ui_state::SketchToolPrefs,
    trim: qymcad_ui_state::TrimTool,
    win: qymcad_ui_state::Windows,
}

impl AtRest {
    fn painting<'a>(&'a self, project: &'a Project, cam: Cam3, sel: Sel) -> Painting<'a> {
        Painting {
            armed: &self.armed,
            boolean: &self.boolean,
            active_path: &self.active_path,
            arr: Default::default(),
            body_giz: &self.body_giz,
            cache: &self.cache,
            cam,
            carr: &self.carr,
            clip: &self.clip,
            cmd: &self.cmd,
            comp_giz: Default::default(),
            cursor: None,
            datum: &self.datum,
            draft: Default::default(),
            chamfer: Default::default(),
            edges: &self.edges,
            face_arrow_drag: None,
            feat: Default::default(),
            gpu_ok: false,
            gsel: &self.gsel,
            hole: Default::default(),
            hover: Default::default(),
            inline: Default::default(),
            interference: &self.interference,
            joint: &self.joint,
            live: &self.live,
            loft: &self.loft,
            m3: &self.m3,
            mirror: &self.mirror,
            mode_3d: true,
            pat: Default::default(),
            pending_import: &self.pending_import,
            picking: Default::default(),
            prim: Default::default(),
            project,
            regen: &self.regen,
            repl_surface: None,
            rev: Default::default(),
            rot: &self.rot,
            scheme: &self.scheme,
            section: &self.section,
            snap_hint: None,
            sel,
            sel_sk: &self.sel_sk,
            set: &self.set,
            sk_pat: Default::default(),
            sketch_hidden: &self.sketch_hidden,
            sketch_ses: Default::default(),
            split: &self.split,
            stitch_parts: &self.stitch_parts,
            recognise: &self.recognise,
            sweep: Default::default(),
            thread: Default::default(),
            tool: &self.tool,
            tool_prefs: &self.tool_prefs,
            trim: &self.trim,
            view: Default::default(),
            view_dragging: false,
            win: &self.win,
            workbench: qymcad_ui_state::Workbench::Part, // the bodies, as the Part workbench shows them
        }
    }
}

/// The selection that lights `lit`, as the window's tree selects a body or a part.
fn selection(project: &Project, lit: Lit) -> Sel {
    match lit {
        Lit::Nothing => Sel::None,
        Lit::Body(b) => project.mesh_index(b).map_or(Sel::None, Sel::Mesh),
        Lit::Part(p) => project.components.iter().position(|c| c.id == p).map_or(Sel::None, Sel::Component),
    }
}

/// THE PICTURE as RGBA, or `None` when there is nothing to draw it at (a size of nothing). `lines` are drawn over the
/// bodies when the look asks for edges.
pub fn draw(project: &Project, look: &Look, lines: &[Line]) -> Option<egui::ColorImage> {
    let rest = AtRest {
        set: Settings { projection: Projection::Ortho, ..Settings::default() },
        scheme: qymcad_ui_state::SchemeUi { pal: qymcad_scheme::light(), ..Default::default() },
        ..Default::default()
    };
    let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(look.size.width as f32, look.size.height as f32));
    let qymcad_render::Angles { yaw, pitch } = qymcad_render::dir_to_angles(look.view.from());
    let mut cam = Cam3 { yaw, pitch, ..Cam3::default() };
    qymcad_render::fit3d(&mut cam, project, rect);
    frame_tightly(&mut cam, project, rect);
    let painting = rest.painting(project, cam, selection(project, look.lit));
    let basis = cam.basis();
    let mut raster = qymcad_render::rasterize_3d_with_depth(&painting, rect, &basis, 1.0, 1.0)?;
    if look.edges == Edges::Drawn {
        let screen = qymcad_ui_state::Screen { cam: &cam, set: &rest.set, rect, basis: &basis };
        // an edge lies on the faces it bounds: it stands in front of them by this much, a two-hundredth of the
        // frame - less than any wall it could be hidden behind, more than the slope of a face across one pixel
        let pen = Pen { lead: f64::from(rect.width().min(rect.height()) / cam.scale) * 0.005, colour: rest.scheme.pal.edge_idle() };
        for line in lines {
            for pair in line.points.windows(2) {
                lay_segment(&mut raster, &Segment { from: screen.at(pair[0]), to: screen.at(pair[1]) }, &pen);
            }
        }
    }
    // the bodies come out of the rasteriser on a transparent ground; a picture handed to a reader is laid on the
    // ground of the window's 3D view
    let ground = rest.scheme.pal.viewport_bg();
    let mut img = raster.image;
    for p in &mut img.pixels {
        *p = over(*p, ground);
    }
    Some(img)
}

/// The share of the picture the model fills along its tighter side.
const FILL: f64 = 0.9;

/// THE MODEL FILLS THE PICTURE. The window's fit leaves the model 0.55 of the shorter side, room for the gizmos and
/// the view cube a picture has none of; measured on a plate from above, it took a quarter of the frame. Here the
/// bodies are measured as they fall on the screen - along the camera's right and up - and framed to `FILL` of the
/// side they bound first.
fn frame_tightly(cam: &mut Cam3, project: &Project, rect: Rect) {
    let (right, up, _) = cam.basis();
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    let consumed = project.consumed_bodies();
    for b in project.bodies.iter().filter(|b| b.visible && !consumed.contains(&b.id)) {
        let wt = project.body_world_transform(b.id);
        for v in &b.mesh.verts {
            let w = qymcad_core::feature::apply12(&wt, [v.x, v.y, v.z]);
            let on = [dot(w, right), dot(w, up)];
            for k in 0..2 {
                lo[k] = lo[k].min(on[k]);
                hi[k] = hi[k].max(on[k]);
            }
        }
    }
    let span = [hi[0] - lo[0], hi[1] - lo[1]];
    if !(span[0].is_finite() && span[1].is_finite()) || span[0].max(span[1]) < 1e-9 {
        return; // nothing to measure, or a single point: the window's fit stands
    }
    // the middle of the bodies on the screen, back in the world: the target keeps its own depth along the view
    let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    let t = cam.target;
    let shift = [mid[0] - dot(t, right), mid[1] - dot(t, up)];
    cam.target = [t[0] + right[0] * shift[0] + up[0] * shift[1], t[1] + right[1] * shift[0] + up[1] * shift[1], t[2] + right[2] * shift[0] + up[2] * shift[1]];
    let fits = |px: f32, along: f64| f64::from(px) / along.max(1e-9);
    cam.scale = (fits(rect.width(), span[0]).min(fits(rect.height(), span[1])) * FILL) as f32;
    cam.fit = cam.scale;
}

/// A piece of an edge on the screen: each end as `Screen::at` gives it, the point and its world depth.
struct Segment {
    from: (egui::Pos2, f64),
    to: (egui::Pos2, f64),
}

/// How an edge is laid: in what colour, and how far it stands in front of the faces it bounds.
struct Pen {
    lead: f64,
    colour: Color32,
}

/// One piece of an edge laid into the frame, pixel by pixel, where no body stands in front of it by more than the
/// pen's lead.
fn lay_segment(raster: &mut qymcad_render::Raster, s: &Segment, pen: &Pen) {
    let [w, h] = raster.image.size;
    let (a, b) = (s.from.0, s.to.0);
    let steps = (b.x - a.x).abs().max((b.y - a.y).abs()).ceil().max(1.0) as usize;
    for k in 0..=steps {
        let t = k as f32 / steps as f32;
        let x = (a.x + (b.x - a.x) * t).floor();
        let y = (a.y + (b.y - a.y) * t).floor();
        if x < 0.0 || y < 0.0 || x as usize >= w || y as usize >= h {
            continue;
        }
        let idx = y as usize * w + x as usize;
        let depth = s.from.1 + (s.to.1 - s.from.1) * f64::from(t);
        if raster.scale.at(depth - pen.lead) <= raster.depth[idx] {
            raster.image.pixels[idx] = pen.colour;
        }
    }
}

/// A pixel laid over the ground by its own alpha (egui keeps colours premultiplied).
fn over(top: Color32, ground: Color32) -> Color32 {
    let rest = 255 - u16::from(top.a());
    let mix = |t: u8, g: u8| (u16::from(t) + u16::from(g) * rest / 255).min(255) as u8;
    Color32::from_rgb(mix(top.r(), ground.r()), mix(top.g(), ground.g()), mix(top.b(), ground.b()))
}

/// THE PICTURE of the document as PNG bytes, its edges taken from the bodies' shapes.
pub fn png(doc: &qymcad_doc::DocEngine, look: &Look) -> Option<Vec<u8>> {
    let lines = if look.edges == Edges::Drawn { lines(doc) } else { Vec::new() };
    qymcad_render::color_image_to_png(&draw(doc.project(), look, &lines)?)
}

/// Bytes in base64 (RFC 4648, with padding): the form the protocol carries a picture in.
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for k in 0..4 {
            if k <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_as_the_standard_writes_it() {
        // the test vectors of RFC 4648, section 10
        struct Vector {
            plain: &'static str,
            coded: &'static str,
        }
        let vectors = [
            Vector { plain: "", coded: "" },
            Vector { plain: "f", coded: "Zg==" },
            Vector { plain: "fo", coded: "Zm8=" },
            Vector { plain: "foo", coded: "Zm9v" },
            Vector { plain: "foob", coded: "Zm9vYg==" },
            Vector { plain: "fooba", coded: "Zm9vYmE=" },
            Vector { plain: "foobar", coded: "Zm9vYmFy" },
        ];
        for v in vectors {
            assert_eq!(base64(v.plain.as_bytes()), v.coded, "{:?}", v.plain);
        }
    }
}
