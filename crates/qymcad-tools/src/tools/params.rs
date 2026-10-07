//! THE PARAMETERS OF THE DOCUMENT: named values (`w = 40`, `wall = w/20`) that dimensions and features read by name.
//! A changed value rebuilds what reads it, as the parameter table of the window does, and the change is one step.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};
use crate::tools::doc::{answer, outcome};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetArgs {
    name: String,
    expr: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteArgs {
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Nothing {}

/// The place of the parameter called `name`, whatever its case: `W` and `w` are one name in a formula.
fn index_of(ctx: &Ctx, name: &str) -> Option<usize> {
    ctx.doc.project().parameters.iter().position(|p| p.name.eq_ignore_ascii_case(name))
}

fn parameter(ctx: &Ctx, i: usize) -> Value {
    let p = &ctx.doc.project().parameters[i];
    json!({ "name": p.name, "expr": p.expr, "value": p.value })
}

/// The name is fit for a formula, and belongs to nothing else: a named dimension already carrying it would make
/// a bare name in a formula mean two things.
fn fit_name(ctx: &Ctx, name: &str) -> Result<(), Refusal> {
    if qymcad_core::drivers::check_ident(name).is_err() {
        return Err(Refusal::new("unfit-name", &format!("The name \"{name}\" cannot be used in a formula: letters, digits and _ only, not starting with a digit."), Stage::Validate));
    }
    let holder = ctx.doc.project().drivers_shown(&qymcad_i18n::name).into_iter().find(|d| d.name.eq_ignore_ascii_case(name.trim()));
    match holder {
        Some(owner) => Err(Refusal::new("taken-name", &format!("The name \"{name}\" belongs to a dimension in {}.", owner.path), Stage::Validate).with_hint("Choose another name.")),
        None => Ok(()),
    }
}

pub const SET_PARAMETER: Tool = Tool {
    name: "set_parameter",
    description: "Set a parameter to an expression (\"40\", \"w/2 + 1\", \"2*pi*r\"), adding it when there is none by that name. Whatever reads it is rebuilt; the answer gives its value, the bodies measured and every feature the change turned red. One undo step.",
    schema: || {
        json!({ "type": "object", "properties": {
            "name": { "type": "string", "description": "Letters, digits and _, not starting with a digit; case does not matter." },
            "expr": { "type": "string", "description": "A number or an expression over other parameters, named dimensions, functions (sin, sqrt, ...) and pi." },
        }, "required": ["name", "expr"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SetArgs = tool::args(arguments)?;
        let name = a.name.trim().to_string();
        let found = index_of(ctx, &name);
        if found.is_none() {
            fit_name(ctx, &name)?;
        }
        // the expression is read before the document changes, so a typo is a refusal rather than a red document; it is
        // read without its own name, since `h = h * 2` has no value - the evaluation runs it round to a fixed point and
        // lands on 5120 for a 10
        let mut names = ctx.doc.project().param_map();
        names.remove(&name.to_lowercase());
        if let Err(e) = qymcad_core::expr::eval(&a.expr, &names) {
            let words = qymcad_i18n::error_words::error_text(&qymcad_core::errors::CoreError::Expr(e.clone()));
            return Err(Refusal::new(e.key(), &words, Stage::Validate).with_hint("Read list_parameters for the names there are."));
        }
        let step = if found.is_some() { "par-edit-step" } else { "win-add-param" };
        let i = ctx
            .doc
            .edit(step, |p| {
                let i = found.unwrap_or_else(|| {
                    p.parameters.push(qymcad_core::model::Param { name: name.clone(), expr: String::new(), value: 0.0 });
                    p.parameters.len() - 1
                });
                p.parameters[i].expr = a.expr.trim().to_string();
                qymcad_doc::params::settle(p);
                Ok(i)
            })
            .map_err(tool::doc_refusal)?;
        Ok(outcome(ctx, answer("parameter", parameter(ctx, i))))
    },
};

pub const DELETE_PARAMETER: Tool = Tool {
    name: "delete_parameter",
    description: "Delete a parameter. Whatever still reads it stands red afterwards, and the answer names it; undo brings the parameter back.",
    schema: || {
        json!({ "type": "object", "properties": {
            "name": { "type": "string" },
        }, "required": ["name"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: DeleteArgs = tool::args(arguments)?;
        let Some(i) = index_of(ctx, a.name.trim()) else {
            return Err(Refusal::new("no-parameter", &format!("There is no parameter \"{}\".", a.name.trim()), Stage::Validate).with_hint("Read list_parameters."));
        };
        let gone = parameter(ctx, i);
        ctx.doc
            .edit("par-delete-step", |p| {
                p.parameters.remove(i);
                qymcad_doc::params::settle(p);
                Ok(())
            })
            .map_err(tool::doc_refusal)?;
        Ok(outcome(ctx, answer("deleted", gone)))
    },
};

pub const LIST_PARAMETERS: Tool = Tool {
    name: "list_parameters",
    description: "Every name a formula can read: the parameters with their expressions and values, and the named dimensions of sketches and features with where they live.",
    schema: || json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    call: |ctx: &mut Ctx, arguments: Value| {
        let Nothing {} = tool::args(arguments)?;
        let p = ctx.doc.project();
        let parameters: Vec<Value> = (0..p.parameters.len()).map(|i| parameter(ctx, i)).collect();
        let drivers: Vec<Value> =
            p.drivers_shown(&qymcad_i18n::name).into_iter().filter(|d| !d.path.is_empty()).map(|d| json!({ "name": d.name, "where": d.path, "value": d.value, "ambiguous": d.ambiguous })).collect();
        let mut a = Answer::new();
        a.insert("parameters".into(), json!(parameters));
        a.insert("dimensions".into(), json!(drivers));
        Ok(a)
    },
};
