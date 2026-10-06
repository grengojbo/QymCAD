//! LOOKING AT THE MODEL: the document drawn from a side, as a picture the model can see.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::BodyRef;
use crate::picture::{self, Edges, Lit, Look, Size, View};
use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool, PICTURE};

/// The bounds of a side of a picture, in pixels: under 64 nothing can be made out, over 2048 the picture outweighs
/// what it shows.
const SIDE: std::ops::RangeInclusive<u32> = 64..=2048;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderArgs {
    #[serde(default)]
    view: View,
    width: Option<u32>,
    height: Option<u32>,
    highlight: Option<BodyRef>,
    #[serde(default = "drawn")]
    edges: bool,
}

fn drawn() -> bool {
    true
}

/// The picture of the document as an answer: the PNG for the protocol to carry as an image, and what it shows.
pub fn picture_answer(ctx: &Ctx, look: &Look) -> Result<Answer, Refusal> {
    let project = ctx.doc.project();
    if project.bodies.is_empty() {
        return Err(Refusal::new("no-body", "The document has no body to draw.", Stage::Validate).with_hint("Lay a body first: a primitive, or a sketch extruded."));
    }
    let png = picture::png(&ctx.doc, look).ok_or_else(|| Refusal::new("bad-size", "The picture could not be drawn at that size.", Stage::Validate))?;
    let mut a = Answer::new();
    a.insert("view".into(), json!(look.view));
    a.insert("size".into(), json!({ "width": look.size.width, "height": look.size.height }));
    a.insert(PICTURE.into(), json!(picture::base64(&png)));
    Ok(a)
}

pub const RENDER: Tool = Tool {
    name: "render",
    description: "Draw the document as a picture (PNG) to look at the result: view iso (front-right-above, the default), top, bottom, front (-Y), back, left, right; width and height in pixels (800 x 600 by default, 64..2048); highlight lights a body ({\"body\": key} or {\"part\": key}) as the window lights a selection; edges (true by default) draws the sharp edges of the bodies, hidden ones left out. The camera is fitted to the whole model. Sketch lines are not drawn.",
    schema: || {
        json!({
            "type": "object",
            "properties": {
                "view": { "type": "string", "enum": ["iso", "top", "bottom", "front", "back", "left", "right"], "default": "iso" },
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
        let size = Size { width: a.width.unwrap_or(800), height: a.height.unwrap_or(600) };
        for side in [size.width, size.height] {
            if !SIDE.contains(&side) {
                return Err(Refusal::new("bad-size", &format!("A side of {side} px is out of {}..{}.", SIDE.start(), SIDE.end()), Stage::Validate));
            }
        }
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
        picture_answer(ctx, &Look { view: a.view, size, lit, edges })
    },
};
