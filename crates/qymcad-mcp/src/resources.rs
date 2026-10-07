//! THE DOCUMENT AS RESOURCES: what a client may read without calling a tool - the whole document, and one feature by
//! its key - in the same JSON the tools answer with. What is there is read in `qymcad_tools::reading`, the same for a
//! document of the program's own and one the window lends.

use serde_json::{json, Value};

use qymcad_tools::reading::{DOCUMENT, FEATURE};

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
