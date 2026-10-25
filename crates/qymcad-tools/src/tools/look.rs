//! LOOKING AT THE MODEL: the document drawn from a side, as a picture the model can see.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::BodyRef;
use crate::picture::{self, Edges, Lit, Look, Size, View};
use crate::tool::{self, Answer, Ctx, Refusal, Seen, Stage, Tool, PICTURE};

/// The bounds of a side of a picture, in pixels: under 64 nothing can be made out, over 2048 the picture outweighs
/// what it shows.
const SIDE: std::ops::RangeInclusive<u32> = 64..=2048;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderArgs {
    view: Option<View>,
    width: Option<u32>,
    height: Option<u32>,
    highlight: Option<BodyRef>,
    #[serde(default = "drawn")]
    edges: bool,
}

fn drawn() -> bool {
    true
}

/// A size of a picture within `SIDE`, or refused.
fn checked(size: Size) -> Result<Size, Refusal> {
    for side in [size.width, size.height] {
        if !SIDE.contains(&side) {
            return Err(Refusal::new("bad-size", &format!("A side of {side} px is out of {}..{}.", SIDE.start(), SIDE.end()), Stage::Validate));
        }
    }
    Ok(size)
}

/// A picture is drawn only of a document with a body, and the window's view only through a window.
fn drawable(ctx: &Ctx, look: &Look) -> Result<(), Refusal> {
    if ctx.doc.project().bodies.is_empty() {
        return Err(Refusal::new("no-body", "The document has no body to draw.", Stage::Validate).with_hint("Lay a body first: a primitive, or a sketch extruded."));
    }
    if look.view == View::Window && look.window.is_none() {
        let missed = match &ctx.seen {
            Seen::NoWindow(missed) => missed.clone(),
            Seen::Window(_) => crate::tool::Missed { why: "the window gave no eye".into() },
        };
        return Err(window_only(&missed));
    }
    Ok(())
}

/// The picture of the document as an answer: the PNG for the protocol to carry as an image, and what it shows.
pub fn picture_answer(ctx: &Ctx, look: &Look) -> Result<Answer, Refusal> {
    drawable(ctx, look)?;
    let shot = picture::png(&ctx.doc, look).ok_or_else(|| Refusal::new("bad-size", "The picture could not be drawn at that size.", Stage::Validate))?;
    let mut a = Answer::new();
    a.insert("view".into(), json!(look.view));
    a.insert("size".into(), json!({ "width": look.size.width, "height": look.size.height }));
    a.insert("filled".into(), json!((shot.filled * 1000.0).round() / 1000.0));
    a.insert(PICTURE.into(), json!(picture::base64(&shot.png)));
    Ok(a)
}

fn window_only(missed: &crate::tool::Missed) -> Refusal {
    Refusal::new("no-window", &format!("The view of the window is drawn only in the open QymCAD window, and this session works on a document of its own: {}.", missed.why), Stage::Window)
        .with_hint("Name a side instead: iso, top, bottom, front, back, left, right.")
}

/// THE SIDE A PICTURE IS DRAWN FROM when none is named: in the open window, as the person sees it; with no window, the
/// window's opening view.
pub fn default_view(ctx: &Ctx) -> View {
    match ctx.seen {
        Seen::Window(_) => View::Window,
        Seen::NoWindow(_) => View::Iso,
    }
}

/// The window's eye, when there is a window.
pub fn eye(ctx: &Ctx) -> Option<picture::Eye> {
    match &ctx.seen {
        Seen::Window(w) => Some(w.eye),
        Seen::NoWindow(_) => None,
    }
}

pub const RENDER: Tool = Tool {
    name: "render",
    description: "Draw the document as a picture (PNG) to look at the result: view iso (front-right-above), top, bottom, front (-Y), back, left, right, or window - as the person sees it in the open QymCAD window, their camera as it stands (the default there; iso is the default with no window); width and height in pixels (800 x 600 by default, 64..2048); highlight lights a body ({\"body\": key} or {\"part\": key}) as the window lights a selection; edges (true by default) draws the sharp edges of the bodies, hidden ones left out. A side is fitted to the whole model. filled tells the share of the picture the model covers - 0 means nothing is in sight. Sketch lines are not drawn.",
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "view": { "type": "string", "enum": ["iso", "top", "bottom", "front", "back", "left", "right", "window"] },
                "width": { "type": "integer", "minimum": SIDE.start(), "maximum": SIDE.end(), "default": 800 },
                "height": { "type": "integer", "minimum": SIDE.start(), "maximum": SIDE.end(), "default": 600 },
                "highlight": crate::args::body_schema(),
                "edges": { "type": "boolean", "default": true },
            },
            "additionalProperties": false
        })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: RenderArgs = tool::args(arguments)?;
        let size = checked(Size { width: a.width.unwrap_or(800), height: a.height.unwrap_or(600) })?;
        let lit = match a.highlight {
            None => Lit::Nothing,
            // a part is lit whole, however many bodies it stands as
            Some(BodyRef::Part(p)) => {
                if !ctx.doc.project().components.iter().any(|c| c.id == p && c.id != ctx.doc.project().root) {
                    return Err(Refusal::new("no-part", &format!("There is no part {p}."), Stage::Validate).with_hint("Read get_document for the parts there are."));
                }
                Lit::Part(p)
            }
            Some(b) => Lit::Body(b.resolve(ctx.doc.project())?),
        };
        let edges = if a.edges { Edges::Drawn } else { Edges::Left };
        let view = a.view.unwrap_or_else(|| default_view(ctx));
        picture_answer(ctx, &Look { view, size, lit, edges, window: eye(ctx) })
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PointAtArgs {
    view: Option<View>,
    width: Option<u32>,
    height: Option<u32>,
    x: f64,
    y: f64,
}

pub const POINT_AT: Tool = Tool {
    name: "point_at",
    description: "What a pixel of a picture shows: give the view, width and height render drew with (the same defaults) and a pixel x, y of that picture (0, 0 at the top left), and get what the window's pointer would catch there - body, part, the face as list_faces reads it (key, kind, centre, normal, a cylinder's radius and axis; on a body made of a mesh the kind is mesh - a flat strip), world (the point hit, in world coordinates) and at (the same point in the body's own frame - the at recognise_feature and fit_at_point take). The nearest face drawn there, facing the eye. Nothing is changed. The ground of the picture is refused: look again at where the model is drawn.",
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "view": { "type": "string", "enum": ["iso", "top", "bottom", "front", "back", "left", "right", "window"] },
                "width": { "type": "integer", "minimum": SIDE.start(), "maximum": SIDE.end(), "default": 800 },
                "height": { "type": "integer", "minimum": SIDE.start(), "maximum": SIDE.end(), "default": 600 },
                "x": { "type": "number", "description": "Pixels from the left edge of the picture." },
                "y": { "type": "number", "description": "Pixels from the top edge of the picture." },
            },
            "required": ["x", "y"],
            "additionalProperties": false
        })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: PointAtArgs = tool::args(arguments)?;
        let size = checked(Size { width: a.width.unwrap_or(800), height: a.height.unwrap_or(600) })?;
        if !(0.0..f64::from(size.width)).contains(&a.x) || !(0.0..f64::from(size.height)).contains(&a.y) {
            return Err(Refusal::new("off-picture", &format!("The pixel {}, {} is off a picture of {} x {}.", a.x, a.y, size.width, size.height), Stage::Validate));
        }
        let view = a.view.unwrap_or_else(|| default_view(ctx));
        let look = Look { view, size, lit: Lit::Nothing, edges: Edges::Left, window: eye(ctx) };
        drawable(ctx, &look)?;
        let project = ctx.doc.project();
        let hit = picture::under(project, &look, egui::pos2(a.x as f32, a.y as f32))
            .ok_or_else(|| Refusal::new("nothing-there", &format!("Nothing of the model is drawn at {}, {}: the pixel shows the ground.", a.x, a.y), Stage::Resolve).with_hint("Look at the picture again and give a pixel on the model."))?;
        let face = project
            .mesh_index(hit.body)
            .and_then(|mi| project.bodies.get(mi))
            .and_then(|b| b.faces.iter().find(|f| f.id == hit.face))
            .ok_or_else(|| Refusal::new("no-face", &format!("Body {} has no face {}.", hit.body, hit.face), Stage::Resolve))?;
        let own = qymcad_core::feature::mat_inv12(&project.body_world_transform(hit.body));
        let round = crate::tools::query::round;
        let mut out = Answer::new();
        out.insert("body".into(), json!(hit.body));
        out.insert("part".into(), json!(project.body_owner(hit.body)));
        // a body brought in as a mesh has no shape: its face is a strip, told without what only the kernel knows
        let entry = match ctx.doc.shape(hit.body) {
            Some(shape) => crate::tools::query::face_entry(project, shape, hit.body, face),
            None => crate::tools::query::mesh_face_entry(project, hit.body, face),
        };
        out.insert("face".into(), entry);
        out.insert("world".into(), json!(hit.world.map(round)));
        out.insert("at".into(), json!(qymcad_core::feature::apply12(&own, hit.world).map(round)));
        Ok(out)
    },
};
