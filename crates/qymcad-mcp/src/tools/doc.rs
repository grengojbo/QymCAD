//! THE DOCUMENT AS A WHOLE: a new one, one opened from a `.qcad` file, the one written back, and the account of what
//! it holds.

use qymcad_doc::report::Report;
use qymcad_doc::DocEngine;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::paths::{self, Existing};
use crate::tool::{self, Answer, Ctx, Refusal, Stage, Tool};

/// What a new document starts with.
#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum Template {
    /// The root assembly and one part in it, active - as the window starts.
    #[default]
    Part,
    /// The root assembly alone.
    Empty,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewArgs {
    #[serde(default)]
    template: Template,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenArgs {
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveArgs {
    path: Option<String>,
    #[serde(default)]
    overwrite: bool,
}

/// How much of the document the account gives.
#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum Detail {
    /// The bodies standing on their own.
    #[default]
    Summary,
    /// Every body, those taken into another one too.
    Full,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GetArgs {
    #[serde(default)]
    detail: Detail,
}

fn coded(e: &qymcad_core::errors::CoreError) -> Value {
    json!({ "code": e.key(), "message": qymcad_i18n::error_words::error_text(e) })
}

fn feature(f: &qymcad_doc::report::Feature) -> Value {
    json!({
        "key": f.key, "name": f.name, "kind": f.kind, "part": f.part, "suppressed": f.suppressed, "bodies": f.bodies,
        "error": f.error.as_ref().map(coded), "warning": f.warning.as_ref().map(coded),
    })
}

fn body(b: &qymcad_doc::report::Body) -> Value {
    json!({
        "id": b.id, "name": b.name, "part": b.part, "volume": b.volume, "area": b.area, "min": b.min, "max": b.max,
        "faces": b.faces, "edges": b.edges, "visible": b.visible, "consumed": b.consumed, "sheet": b.sheet,
    })
}

/// The account of the document as the model reads it; the names in English.
fn document(ctx: &Ctx, detail: Detail) -> Value {
    let r: Report = ctx.doc.document(&qymcad_i18n::name);
    let parts: Vec<Value> = r.parts.iter().map(|p| json!({ "key": p.key, "name": p.name, "assembly": p.assembly, "parent": p.parent, "visible": p.visible })).collect();
    let features: Vec<Value> = r.features.iter().map(feature).collect();
    let bodies: Vec<Value> = r.bodies.iter().filter(|b| matches!(detail, Detail::Full) || !b.consumed).map(body).collect();
    let parameters: Vec<Value> = r.parameters.iter().map(|p| json!({ "name": p.name, "expr": p.expr, "value": p.value })).collect();
    let history = ctx.doc.history();
    json!({
        "path": ctx.path,
        "parts": parts,
        "features": features,
        "bodies": bodies,
        "parameters": parameters,
        "undo": history.undo_names(),
        "redo": history.redo_names(),
    })
}

/// WHERE THE DOCUMENT STANDS AFTER AN ACTION, beside the action's own answer: the bodies standing on their own,
/// measured, and every feature that stands red or short - so the model reads what its action did without asking.
pub fn outcome(ctx: &Ctx, mut a: Answer) -> Answer {
    let r: Report = ctx.doc.document(&qymcad_i18n::name);
    let bodies: Vec<Value> = r.bodies.iter().filter(|b| !b.consumed).map(body).collect();
    let red: Vec<Value> = r.features.iter().filter(|f| f.error.is_some() || f.warning.is_some()).map(feature).collect();
    a.insert("bodies".into(), json!(bodies));
    a.insert("red".into(), json!(red));
    a
}

pub fn answer(key: &str, value: Value) -> Answer {
    let mut a = Answer::new();
    a.insert(key.into(), value);
    a
}

pub const NEW_PROJECT: Tool = Tool {
    name: "new_project",
    description: "Start a new document in place of the open one, which is dropped unsaved. template \"part\" (the default) gives one part to model in; \"empty\" gives the root assembly alone.",
    schema: || {
        json!({ "type": "object", "properties": {
            "template": { "type": "string", "enum": ["part", "empty"], "default": "part" },
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: NewArgs = tool::args(arguments)?;
        ctx.doc = match a.template {
            Template::Part => DocEngine::blank(),
            Template::Empty => {
                let mut p = qymcad_core::model::Project::default();
                p.new_empty_document();
                DocEngine::new(p)
            }
        };
        ctx.path = None;
        Ok(answer("document", document(ctx, Detail::Summary)))
    },
};

pub const OPEN_PROJECT: Tool = Tool {
    name: "open_project",
    description: "Open a .qcad document in place of the open one, which is dropped unsaved. The path is absolute, from the server's folder, or from ~. Answers with the account of the document.",
    schema: || {
        json!({ "type": "object", "properties": {
            "path": { "type": "string", "description": "The .qcad file." },
        }, "required": ["path"], "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: OpenArgs = tool::args(arguments)?;
        let path = paths::resolve(&a.path);
        paths::has_extension(&path, &["qcad"])?;
        let path = path.to_string_lossy().into_owned();
        ctx.doc = DocEngine::open(&path).map_err(tool::doc_refusal)?;
        ctx.path = Some(path);
        Ok(answer("document", document(ctx, Detail::Summary)))
    },
};

pub const SAVE_PROJECT: Tool = Tool {
    name: "save_project",
    description: "Write the document to a .qcad file with its live bodies. Without path it goes to the file it was opened from or last saved to. A file that is there already, other than the document's own, is written over only with overwrite: true.",
    schema: || {
        json!({ "type": "object", "properties": {
            "path": { "type": "string", "description": "The .qcad file; the document's own file when left out." },
            "overwrite": { "type": "boolean", "default": false },
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: SaveArgs = tool::args(arguments)?;
        let Some(raw) = a.path.or_else(|| ctx.path.clone()) else {
            return Err(Refusal::new("no-path", "The document has no file yet.", Stage::Io).with_hint("Pass path."));
        };
        let path = paths::resolve(&raw);
        paths::has_extension(&path, &["qcad"])?;
        let target = path.to_string_lossy().into_owned();
        // the document's own file is the one it is saved to; only another file needs the leave to write over it
        let existing = if ctx.path.as_deref() == Some(target.as_str()) { Existing::WriteOver } else { Existing::from_overwrite(a.overwrite) };
        paths::may_write(&path, existing)?;
        ctx.doc.save(&target, &qymcad_doc::clock::now_iso8601()).map_err(tool::doc_refusal)?;
        ctx.path = Some(target.clone());
        Ok(answer("path", json!(target)))
    },
};

pub const GET_DOCUMENT: Tool = Tool {
    name: "get_document",
    description: "The account of the document: its parts, its timeline with every feature that stands red and why, its bodies measured (volume mm^3, area mm^2, box mm, faces, edges), its parameters, and the steps undo and redo would take. detail \"full\" adds the bodies taken into another one.",
    schema: || {
        json!({ "type": "object", "properties": {
            "detail": { "type": "string", "enum": ["summary", "full"], "default": "summary" },
        }, "additionalProperties": false })
    },
    call: |ctx: &mut Ctx, arguments: Value| {
        let a: GetArgs = tool::args(arguments)?;
        Ok(answer("document", document(ctx, a.detail)))
    },
};
