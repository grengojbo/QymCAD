//! UNDO AND REDO: one step of the document's history back or forward, the same steps the window takes.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};

/// The arguments of a tool that takes none: any field given is refused.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Nothing {}

fn no_arguments() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

/// The step taken: the name it is kept under and its words.
fn step(key: &str, name: &str) -> Answer {
    let mut a = Answer::new();
    a.insert(key.into(), json!({ "step": name, "words": qymcad_i18n::name(name) }));
    a
}

pub const UNDO: Tool = Tool {
    name: "undo",
    description: "Take back the last step: the document returns to what it was before that step, rebuilt.",
    schema: no_arguments,
    call: |ctx: &mut Ctx, arguments: Value| {
        let Nothing {} = tool::args(arguments)?;
        let name = ctx.doc.undo().ok_or_else(|| Refusal::new("nothing-to-undo", "There is no step to undo.", Stage::History))?;
        Ok(step("undone", &name))
    },
};

pub const REDO: Tool = Tool {
    name: "redo",
    description: "Put back the last step taken back by undo, rebuilt. A new step clears what redo could put back.",
    schema: no_arguments,
    call: |ctx: &mut Ctx, arguments: Value| {
        let Nothing {} = tool::args(arguments)?;
        let name = ctx.doc.redo().ok_or_else(|| Refusal::new("nothing-to-redo", "There is no step to redo.", Stage::History))?;
        Ok(step("redone", &name))
    },
};
