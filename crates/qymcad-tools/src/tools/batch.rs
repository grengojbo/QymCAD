//! A BATCH OF OPERATIONS AS ONE STEP: the tools called one after another, each on what the one before it built, and
//! the whole kept as one undo step - or, when any of them is refused, taken back whole, the document as it was before
//! the call.
//!
//! An operation may be named with `as`; a later one reads a field of its answer as `"$name.field"` - `"$base.body"`
//! for the body a box laid, `"$plan.sketch"` for a sketch.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};
use crate::tools::doc::outcome;

/// THE TOOLS A BATCH DOES NOT TAKE: what replaces the document (new, open), what reaches outside it and cannot be
/// taken back with it (save, export), the history itself (undo, redo), and a batch inside a batch.
const NOT_IN_A_BATCH: [&str; 8] = ["new_project", "open_project", "save_project", "export_mesh", "export_cad", "undo", "redo", "apply_ops"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Op {
    tool: String,
    #[serde(default)]
    args: Value,
    #[serde(rename = "as")]
    named: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchArgs {
    ops: Vec<Op>,
    name: Option<String>,
}

/// An answer kept under its name, for the operations after it to read.
struct Named {
    name: String,
    answer: Answer,
}

/// The arguments with every `"$name.field"` replaced by that field of the named answer.
fn filled(args: &Value, named: &[Named]) -> Result<Value, Refusal> {
    Ok(match args {
        Value::String(s) if s.starts_with('$') => {
            let (name, field) = s[1..].split_once('.').ok_or_else(|| Refusal::new("unknown-name", &format!("\"{s}\" names no field: write \"$name.field\"."), Stage::Validate))?;
            let answer = named.iter().rev().find(|n| n.name == name).map(|n| &n.answer).ok_or_else(|| {
                Refusal::new("unknown-name", &format!("No operation before this one is named \"{name}\"."), Stage::Validate).with_hint("Name an operation with \"as\" before reading its answer.")
            })?;
            answer
                .get(field)
                .cloned()
                .ok_or_else(|| Refusal::new("unknown-name", &format!("The answer of \"{name}\" has no field \"{field}\"; it has {:?}.", answer.keys().collect::<Vec<_>>()), Stage::Validate))?
        }
        Value::Array(items) => Value::Array(items.iter().map(|v| filled(v, named)).collect::<Result<_, _>>()?),
        Value::Object(map) => Value::Object(map.iter().map(|(k, v)| filled(v, named).map(|v| (k.clone(), v))).collect::<Result<_, _>>()?),
        other => other.clone(),
    })
}

/// The operations run in the open batch; the answers of each, or the refusal of the first that is refused, its place
/// and its tool named.
fn run(ctx: &mut Ctx, ops: &[Op]) -> Result<Vec<Value>, Refusal> {
    let mut named: Vec<Named> = Vec::new();
    let mut done = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let step = i + 1;
        let at = |r: Refusal| Refusal { message: format!("Operation {step} ({}): {}", op.tool, r.message), ..r };
        let found = tool::find(&op.tool).ok_or_else(|| at(Refusal::new("no-tool", &format!("There is no tool {}.", op.tool), Stage::Validate)))?;
        if NOT_IN_A_BATCH.contains(&found.name) {
            return Err(at(Refusal::new("not-in-batch", &format!("{} cannot go into a batch.", found.name), Stage::Validate).with_hint("Call it on its own, before or after the batch.")));
        }
        let args = filled(&op.args, &named).map_err(at)?;
        let answer = (found.call)(ctx, args).map_err(at)?;
        let mut shown = json!({ "op": found.name });
        for key in ["body", "feature", "sketch", "part", "parameter"] {
            if let Some(v) = answer.get(key) {
                shown[key] = v.clone();
            }
        }
        if let Some(name) = &op.named {
            shown["as"] = json!(name);
            named.push(Named { name: name.clone(), answer });
        }
        done.push(shown);
    }
    Ok(done)
}

pub const APPLY_OPS: Tool = Tool {
    name: "apply_ops",
    description: "Run several operations as ONE undo step: ops is a list of {\"tool\": name, \"args\": {...}, \"as\": optional name}. Each runs on what the ones before it built; a later one reads a field of a named one's answer as \"$name.field\" (\"$base.body\", \"$plan.sketch\"). When any is refused, the whole batch is taken back and the document is as it was; the refusal names the operation. Not in a batch: new_project, open_project, save_project, export_*, undo, redo, apply_ops. name is the step's name (Operations by default).",
    schema: || {
        json!({ "type": "object", "properties": {
            "ops": { "type": "array", "minItems": 1, "items": { "type": "object", "properties": {
                "tool": { "type": "string" },
                "args": { "type": "object" },
                "as": { "type": "string" },
            }, "required": ["tool"], "additionalProperties": false } },
            "name": { "type": "string" },
        }, "required": ["ops"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: BatchArgs = tool::args(arguments)?;
        if a.ops.is_empty() {
            return Err(Refusal::new("arguments", "ops is empty: nothing to do.", Stage::Validate));
        }
        let name = a.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| "status-ops-batch".to_string());
        ctx.doc.begin_batch(&name);
        match run(ctx, &a.ops) {
            Ok(done) => {
                ctx.doc.commit_batch();
                let mut out = Answer::new();
                out.insert("step".into(), json!({ "step": name, "words": qymcad_i18n::name(&name) }));
                out.insert("ops".into(), json!(done));
                Ok(outcome(ctx, out))
            }
            Err(refusal) => {
                ctx.doc.abort_batch();
                Err(refusal)
            }
        }
    },
};
