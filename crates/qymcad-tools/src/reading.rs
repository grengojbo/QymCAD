//! THE DOCUMENT READ BY ADDRESS: the whole document, and one feature by its key - in the same JSON the tools answer
//! with. Read here rather than in the program's protocol, because a document lent by the window is read the same way.

use serde_json::{json, Value};

use crate::tool::Ctx;
use crate::tools::doc::{document, feature, Detail};

/// The address of the whole document.
pub const DOCUMENT: &str = "qymcad://document";

/// The head of the address of one feature; its key follows.
pub const FEATURE: &str = "qymcad://feature/";

/// WHY AN ADDRESS READ NOTHING.
#[derive(Debug, PartialEq)]
pub enum Unread {
    /// The address is malformed: its words.
    BadAddress(String),
    /// The address is well formed and nothing is there: its words.
    Nothing(String),
}

/// THE CONTENTS AT `uri`, as the protocol carries them: one text in JSON.
pub fn read(ctx: &Ctx, uri: &str) -> Result<Value, Unread> {
    let text = if uri == DOCUMENT {
        document(ctx, Detail::Summary)
    } else if let Some(key) = uri.strip_prefix(FEATURE) {
        let key: qymcad_core::model::Id = key.parse().map_err(|_| Unread::BadAddress(format!("\"{key}\" is no feature key: a key is a number.")))?;
        let report = ctx.doc.document(&qymcad_i18n::name);
        let found = report.features.iter().find(|f| f.key == key).ok_or_else(|| Unread::Nothing(format!("There is no feature {key}.")))?;
        let mut v = feature(found);
        v["sizes"] = crate::tools::timeline::sizes(ctx.doc.project(), key);
        v
    } else {
        return Err(Unread::Nothing(format!("There is nothing at {uri}.")));
    };
    Ok(json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": text.to_string() }] }))
}

/// A READ AS ONE VALUE, the way it crosses the window's channel: the contents, or why there are none.
pub fn read_as_answer(ctx: &Ctx, uri: &str) -> Value {
    match read(ctx, uri) {
        Ok(contents) => contents,
        Err(Unread::BadAddress(m)) => json!({ "unread": { "kind": "address", "message": m } }),
        Err(Unread::Nothing(m)) => json!({ "unread": { "kind": "nothing", "message": m } }),
    }
}

/// The value [`read_as_answer`] made, read back.
pub fn answer_as_read(answer: Value) -> Result<Value, Unread> {
    let Some(unread) = answer.get("unread") else { return Ok(answer) };
    let message = unread["message"].as_str().unwrap_or_default().to_string();
    Err(if unread["kind"] == "address" { Unread::BadAddress(message) } else { Unread::Nothing(message) })
}
