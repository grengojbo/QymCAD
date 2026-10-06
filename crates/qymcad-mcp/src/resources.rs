//! THE DOCUMENT AS RESOURCES: what a client may read without calling a tool - the whole document, and one feature by
//! its key - in the same JSON the tools answer with.

use serde_json::{json, Value};

use crate::rpc::Fault;
use crate::tool::Ctx;
use crate::tools::doc::{document, feature, Detail};

/// The address of the whole document.
const DOCUMENT: &str = "qymcad://document";

/// The head of the address of one feature; its key follows.
const FEATURE: &str = "qymcad://feature/";

/// The resources there always are.
pub fn listing() -> Value {
    json!([{
        "uri": DOCUMENT,
        "name": "document",
        "title": "The open document",
        "description": "Its parts, features (red ones with the reason), bodies standing on their own with volume, area and bounds, parameters, and the undo history - what get_document answers.",
        "mimeType": "application/json",
    }])
}

/// The resources read by a key.
pub fn templates() -> Value {
    json!([{
        "uriTemplate": format!("{FEATURE}{{key}}"),
        "name": "feature",
        "title": "One feature",
        "description": "One feature of the timeline by its key: its kind, part, bodies, error or warning, and its sizes with the expressions they follow.",
        "mimeType": "application/json",
    }])
}

/// A resource that cannot be read: the fault and its words.
pub struct Missing {
    pub fault: Fault,
    pub message: String,
}

/// THE CONTENTS AT `uri`, as the protocol carries them: one text in JSON.
pub fn read(ctx: &Ctx, uri: &str) -> Result<Value, Missing> {
    let text = if uri == DOCUMENT {
        document(ctx, Detail::Summary)
    } else if let Some(key) = uri.strip_prefix(FEATURE) {
        let key: qymcad_core::model::Id = key.parse().map_err(|_| Missing { fault: Fault::InvalidParams, message: format!("\"{key}\" is no feature key: a key is a number.") })?;
        let report = ctx.doc.document(&qymcad_i18n::name);
        let found = report.features.iter().find(|f| f.key == key).ok_or_else(|| Missing { fault: Fault::ResourceNotFound, message: format!("There is no feature {key}.") })?;
        let mut v = feature(found);
        v["sizes"] = crate::tools::timeline::sizes(ctx.doc.project(), key);
        v
    } else {
        return Err(Missing { fault: Fault::ResourceNotFound, message: format!("There is nothing at {uri}.") });
    };
    Ok(json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": text.to_string() }] }))
}
