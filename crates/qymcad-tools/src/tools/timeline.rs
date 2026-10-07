//! PARTS AND THE TIMELINE: a new part to model in, stepping into a part, and the features already laid - deleted,
//! suppressed or given new sizes - each as the window does it, the document rebuilt and every feature the change
//! turned red named in the answer.

use std::collections::BTreeMap;

use qymcad_core::feature::FeatureKind;
use qymcad_core::model::{DimTarget, Id, Project};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::args::Amount;
use crate::tool::{self, Ctx, Refusal, Stage, Tool};
use crate::tools::doc::{answer, outcome};

/// The place of feature `key` in the timeline.
fn place(project: &Project, key: Id) -> Result<usize, Refusal> {
    project
        .timeline
        .iter()
        .position(|n| n.id == key)
        .ok_or_else(|| Refusal::new("no-feature", &format!("There is no feature {key}."), Stage::Validate).with_hint("Read get_document for the features there are."))
}

/// The sizes of a feature as the model reads them: the key, the value, and the expression it follows, if any.
pub fn sizes(project: &Project, key: Id) -> Value {
    let Some(n) = project.timeline.iter().find(|n| n.id == key) else { return json!([]) };
    // the size the rebuild takes: the expression's value where there is one - a number typed for a feature lives there
    // too - otherwise the stored number
    let all: Vec<Value> = n
        .kind
        .dims()
        .into_iter()
        .map(|(k, stored)| {
            let expr = project.feat_dim(key, k).map(str::trim).filter(|e| !e.is_empty());
            let value = expr.and_then(|e| project.eval_expr(e).ok()).unwrap_or(stored);
            let follows = expr.filter(|e| e.parse::<f64>().is_err());
            json!({ "key": k, "value": value, "expr": follows })
        })
        .collect();
    json!(all)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddPartArgs {
    name: Option<String>,
}

pub const ADD_PART: Tool = Tool {
    name: "add_part",
    description: "Add a new part to the root assembly and step into it: what is laid next goes into it. name is optional (Part N by default). For a second piece of a print kept apart from the first.",
    schema: || json!({ "type": "object", "properties": { "name": { "type": "string" } }, "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: AddPartArgs = tool::args(arguments)?;
        let name = a.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        let part = ctx
            .doc
            .edit("status-new-part", |p| {
                let root = p.ensure_root();
                p.set_active_component(Some(root));
                let name = name.unwrap_or_else(|| p.free_part_name());
                let part = p.add_part(name);
                p.set_active_component(Some(part));
                Ok(part)
            })
            .map_err(tool::doc_refusal)?;
        Ok(outcome(ctx, answer("part", json!(part))))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveArgs {
    part: Id,
}

pub const SET_ACTIVE_PART: Tool = Tool {
    name: "set_active_part",
    description: "Step into a part: what is laid next goes into it, and tools without a body work on its body. Not an undo step - where the work goes is not a change of the document.",
    schema: || json!({ "type": "object", "properties": { "part": { "type": "integer" } }, "required": ["part"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: ActiveArgs = tool::args(arguments)?;
        if !ctx.doc.set_active_part(a.part) {
            return Err(Refusal::new("no-part", &format!("There is no part {}.", a.part), Stage::Validate).with_hint("Read get_document for the parts there are."));
        }
        Ok(answer("part", json!(a.part)))
    },
};

/// What becomes of the features standing on a deleted one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Dependents {
    /// They stay: a feature in the middle hands them to its source, a base leaves them red with the reason.
    #[default]
    Stay,
    /// They go with it.
    Go,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteArgs {
    feature: Id,
    #[serde(default)]
    dependents: Dependents,
}

pub const DELETE_FEATURE: Tool = Tool {
    name: "delete_feature",
    description: "Delete a feature (or a sketch, or a datum) from the timeline. dependents \"stay\" (the default): a feature in the middle hands what stands on it to its source, a base leaves it red, and the answer names what turned red; \"go\" deletes everything built on it too. Undo brings it all back.",
    schema: || json!({ "type": "object", "properties": { "feature": { "type": "integer" }, "dependents": { "type": "string", "enum": ["stay", "go"], "default": "stay" } }, "required": ["feature"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: DeleteArgs = tool::args(arguments)?;
        let ti = place(ctx.doc.project(), a.feature)?;
        let node = ctx.doc.project().timeline[ti].clone();
        let go = a.dependents == Dependents::Go;
        let sketch = match node.kind {
            FeatureKind::Sketch { sketch } => Some(sketch),
            _ => None,
        };
        let step = if sketch.is_some() { "status-delete-sketch" } else { "status-delete-feature" };
        ctx.doc
            .edit(step, |p| {
                // the core's tested deletions, the same the window's Delete takes
                match sketch {
                    Some(sid) if go => {
                        let _ = p.delete_sketch_with_dependents(sid);
                    }
                    Some(sid) => p.delete_sketch(sid),
                    None if node.kind.body().is_some() => {
                        let _ = if go { p.delete_feature_with_dependents(node.id) } else { p.delete_feature_op(node.id) };
                    }
                    None => {
                        let _ = p.timeline.remove(ti);
                    }
                }
                Ok(())
            })
            .map_err(tool::doc_refusal)?;
        Ok(outcome(ctx, answer("deleted", json!(a.feature))))
    },
};

/// Whether a feature takes part in the rebuild.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Taking {
    /// Left out of the rebuild, kept in the timeline.
    Suppress,
    /// Taken back in.
    Restore,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SuppressArgs {
    feature: Id,
    state: Taking,
}

pub const SUPPRESS_FEATURE: Tool = Tool {
    name: "suppress_feature",
    description: "Leave a feature out of the rebuild without deleting it (state \"suppress\"), or take it back in (\"restore\") - to try the part without a fillet, say. The answer gives the bodies measured and what turned red.",
    schema: || json!({ "type": "object", "properties": { "feature": { "type": "integer" }, "state": { "type": "string", "enum": ["suppress", "restore"] } }, "required": ["feature", "state"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SuppressArgs = tool::args(arguments)?;
        let ti = place(ctx.doc.project(), a.feature)?;
        let on = a.state == Taking::Suppress;
        let step = if on { "status-op-suppressed" } else { "status-op-enabled" };
        ctx.doc
            .edit(step, |p| {
                let _ = p.set_feature_suppressed(ti, on);
                Ok(())
            })
            .map_err(tool::doc_refusal)?;
        let mut out = answer("feature", json!(a.feature));
        out.insert("suppressed".into(), json!(on));
        Ok(outcome(ctx, out))
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EditArgs {
    feature: Id,
    values: BTreeMap<String, Amount>,
}

pub const EDIT_FEATURE: Tool = Tool {
    name: "edit_feature",
    description: "Give a feature new sizes: values maps a key of the feature (get_document lists the features; this answer lists their keys - height, radius, depth, angle, dx ...) to a number or an expression. A number replaces the size; an expression keeps following its parameters. The document is rebuilt; one undo step.",
    schema: || json!({ "type": "object", "properties": { "feature": { "type": "integer" }, "values": { "type": "object", "additionalProperties": { "oneOf": [ { "type": "number" }, { "type": "string" } ] } } }, "required": ["feature", "values"], "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: EditArgs = tool::args(arguments)?;
        let project = ctx.doc.project();
        let ti = place(project, a.feature)?;
        let keys: Vec<&str> = project.timeline[ti].kind.dims().into_iter().map(|(k, _)| k).collect();
        if a.values.is_empty() {
            return Err(Refusal::new("arguments", "values is empty: nothing to change.", Stage::Validate));
        }
        let mut read = Vec::new();
        for (key, v) in &a.values {
            if !keys.contains(&key.as_str()) {
                return Err(Refusal::new("no-size", &format!("Feature {} has no size \"{key}\"; it has {keys:?}.", a.feature), Stage::Validate));
            }
            read.push(Sized { key: key.clone(), read: v.read(project, key)? });
        }
        ctx.doc
            .edit("par-edit-step", |p| {
                for s in &read {
                    match &s.read.expr {
                        Some(e) => p.set_feat_dim(a.feature, &s.key, e.clone()),
                        // a number lives in the expression of a feature: that is how the rebuild applies it and how the
                        // window's properties show it (`set_dim_target_value`)
                        None => {
                            let _ = p.set_dim_target_value(&DimTarget::Feature { node: a.feature, key: s.key.clone() }, s.read.value);
                        }
                    }
                }
                Ok(())
            })
            .map_err(tool::doc_refusal)?;
        let mut out = answer("feature", json!(a.feature));
        out.insert("sizes".into(), sizes(ctx.doc.project(), a.feature));
        Ok(outcome(ctx, out))
    },
};

/// A size to set: its key and what was given for it.
struct Sized {
    key: String,
    read: crate::args::Read,
}
